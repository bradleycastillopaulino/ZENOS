//! Proveedores de IA. Cada familia de API tiene su adaptador independiente.
//! Para añadir un proveedor compatible con OpenAI basta con añadir una entrada
//! en `PROVIDERS`; para otra API distinta, crear un módulo que implemente `ChatAdapter`.

mod anthropic;
mod openai_compat;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage {
    pub role: String, // "user" | "assistant"
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ApiKind {
    OpenAiCompatible,
    Anthropic,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub id: &'static str,
    pub name: &'static str,
    #[serde(skip)]
    pub kind: ApiKind,
    #[serde(skip)]
    pub base_url: &'static str,
    pub default_model: &'static str,
    pub suggested_models: &'static [&'static str],
    /// Dónde obtener la clave.
    pub key_url: &'static str,
    /// `reasoning_effort` por defecto (modelos de razonamiento). Se omite si falla.
    #[serde(skip)]
    pub reasoning_effort: Option<&'static str>,
}

pub const PROVIDERS: &[ProviderInfo] = &[
    ProviderInfo {
        id: "anthropic",
        name: "Anthropic Claude",
        kind: ApiKind::Anthropic,
        base_url: "https://api.anthropic.com/v1",
        default_model: "claude-haiku-5-5",
        suggested_models: &["claude-haiku-5-5", "claude-sonnet-5-5", "claude-opus-5-5"],
        key_url: "https://console.anthropic.com/settings/keys",
        reasoning_effort: None,
    },
    ProviderInfo {
        id: "openai",
        name: "OpenAI (ChatGPT)",
        kind: ApiKind::OpenAiCompatible,
        base_url: "https://api.openai.com/v1",
        default_model: "gpt-5.4-mini",
        suggested_models: &["gpt-5.4-mini", "gpt-5.4-nano", "gpt-5.6-luna", "chat-latest"],
        key_url: "https://platform.openai.com/api-keys",
        reasoning_effort: Some("low"),
    },
    ProviderInfo {
        id: "gemini",
        name: "Google Gemini",
        kind: ApiKind::OpenAiCompatible,
        base_url: "https://generativelanguage.googleapis.com/v1beta/openai",
        default_model: "gemini-flash-latest",
        suggested_models: &["gemini-flash-latest", "gemini-3.6-flash", "gemini-3.5-flash"],
        key_url: "https://aistudio.google.com/apikey",
        reasoning_effort: Some("low"),
    },
    ProviderInfo {
        id: "groq",
        name: "Groq",
        kind: ApiKind::OpenAiCompatible,
        base_url: "https://api.groq.com/openai/v1",
        default_model: "openai/gpt-oss-20b",
        suggested_models: &["openai/gpt-oss-20b", "openai/gpt-oss-120b"],
        key_url: "https://console.groq.com/keys",
        reasoning_effort: Some("low"),
    },
    ProviderInfo {
        id: "deepseek",
        name: "DeepSeek",
        kind: ApiKind::OpenAiCompatible,
        base_url: "https://api.deepseek.com/v1",
        default_model: "deepseek-chat",
        suggested_models: &["deepseek-chat"],
        key_url: "https://platform.deepseek.com/api_keys",
        reasoning_effort: None,
    },
    ProviderInfo {
        id: "openrouter",
        name: "OpenRouter",
        kind: ApiKind::OpenAiCompatible,
        base_url: "https://openrouter.ai/api/v1",
        default_model: "google/gemini-3.5-flash",
        suggested_models: &["google/gemini-3.5-flash", "anthropic/claude-haiku-5-5", "openai/gpt-5.4-mini"],
        key_url: "https://openrouter.ai/keys",
        reasoning_effort: None,
    },
];

pub fn provider_info(id: &str) -> Option<&'static ProviderInfo> {
    PROVIDERS.iter().find(|p| p.id == id)
}

pub struct ChatRequest<'a> {
    pub model: &'a str,
    pub system: &'a str,
    pub messages: &'a [ChatMessage],
    pub max_tokens: u32,
}

/// Envía la conversación al proveedor y devuelve el texto de la respuesta.
pub async fn chat(
    client: &reqwest::Client,
    provider: &ProviderInfo,
    api_key: &str,
    req: ChatRequest<'_>,
) -> Result<String, String> {
    let answer = match provider.kind {
        ApiKind::OpenAiCompatible => openai_compat::chat(client, provider, api_key, &req).await?,
        ApiKind::Anthropic => anthropic::chat(client, provider, api_key, &req).await?,
    };
    let answer = answer.trim().to_string();
    if answer.is_empty() {
        return Err("La IA devolvió una respuesta vacía.".into());
    }
    Ok(answer)
}

/// Convierte un error HTTP en un mensaje breve y hablable.
pub(crate) fn friendly_http_error(provider: &str, status: u16, body: &str) -> String {
    crate::log::write(&format!("{provider} HTTP {status}: {}", &body.chars().take(400).collect::<String>()));
    match status {
        401 | 403 => format!("La clave de {provider} no es válida."),
        404 => format!("El modelo configurado no existe en {provider}."),
        402 => format!("La cuenta de {provider} no tiene saldo."),
        429 => format!("Se alcanzó el límite de uso de {provider}."),
        500..=599 => format!("El servicio de {provider} no está disponible ahora."),
        _ => format!("Error al consultar {provider}."),
    }
}

pub(crate) fn network_error(provider: &str, e: reqwest::Error) -> String {
    crate::log::write(&format!("{provider} red: {e}"));
    if e.is_timeout() {
        format!("{provider} tardó demasiado en responder.")
    } else {
        "No hay conexión a internet.".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_ids_unique() {
        let mut ids: Vec<_> = PROVIDERS.iter().map(|p| p.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), PROVIDERS.len());
        assert_eq!(PROVIDERS.len(), 6);
    }

    #[test]
    fn friendly_errors() {
        assert!(friendly_http_error("Groq", 401, "").contains("no es válida"));
        assert!(friendly_http_error("Groq", 429, "").contains("límite"));
    }
}
