//! Adaptador para APIs compatibles con OpenAI Chat Completions:
//! OpenAI, Gemini (endpoint OpenAI), Groq, DeepSeek y OpenRouter.

use serde_json::{json, Value};

use super::{friendly_http_error, network_error, ChatRequest, ProviderInfo};

pub(crate) fn build_body(provider: &ProviderInfo, req: &ChatRequest<'_>, with_reasoning: bool) -> Value {
    let mut messages = vec![json!({"role": "system", "content": req.system})];
    for m in req.messages {
        messages.push(json!({"role": m.role, "content": m.content}));
    }
    let mut body = json!({ "model": req.model, "messages": messages });
    // OpenAI usa max_completion_tokens; el resto acepta max_tokens.
    if provider.id == "openai" {
        body["max_completion_tokens"] = json!(req.max_tokens);
    } else {
        body["max_tokens"] = json!(req.max_tokens);
    }
    if with_reasoning {
        if let Some(effort) = provider.reasoning_effort {
            body["reasoning_effort"] = json!(effort);
        }
    }
    body
}

pub(crate) fn extract_answer(v: &Value) -> Option<String> {
    let content = &v["choices"][0]["message"]["content"];
    match content {
        Value::String(s) => Some(s.clone()),
        // Algunos proveedores devuelven partes.
        Value::Array(parts) => Some(
            parts.iter().filter_map(|p| p["text"].as_str()).collect::<Vec<_>>().join(""),
        ),
        _ => None,
    }
}

pub async fn chat(
    client: &reqwest::Client,
    provider: &ProviderInfo,
    api_key: &str,
    req: &ChatRequest<'_>,
) -> Result<String, String> {
    let url = format!("{}/chat/completions", provider.base_url);
    let mut with_reasoning = provider.reasoning_effort.is_some();
    loop {
        let body = build_body(provider, req, with_reasoning);
        let mut rb = client.post(&url).bearer_auth(api_key).json(&body);
        if provider.id == "openrouter" {
            rb = rb.header("X-Title", "AI Star");
        }
        let resp = rb.send().await.map_err(|e| network_error(provider.name, e))?;
        let status = resp.status().as_u16();
        let text = resp.text().await.map_err(|e| network_error(provider.name, e))?;
        if status == 400 && with_reasoning && text.contains("reasoning") {
            // El modelo elegido no admite reasoning_effort: reintentar sin él.
            with_reasoning = false;
            continue;
        }
        if !(200..300).contains(&status) {
            return Err(friendly_http_error(provider.name, status, &text));
        }
        let v: Value = serde_json::from_str(&text).map_err(|_| "Respuesta inválida de la IA.".to_string())?;
        return extract_answer(&v).ok_or_else(|| "La IA devolvió una respuesta vacía.".into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{provider_info, ChatMessage};

    #[test]
    fn body_shape() {
        let msgs = vec![ChatMessage { role: "user".into(), content: "hola".into() }];
        let req = ChatRequest { model: "m", system: "sys", messages: &msgs, max_tokens: 100 };
        let b = build_body(provider_info("openai").unwrap(), &req, true);
        assert_eq!(b["messages"][0]["role"], "system");
        assert_eq!(b["messages"][1]["content"], "hola");
        assert_eq!(b["max_completion_tokens"], 100);
        assert_eq!(b["reasoning_effort"], "low");
        let b = build_body(provider_info("deepseek").unwrap(), &req, true);
        assert_eq!(b["max_tokens"], 100);
        assert!(b.get("reasoning_effort").is_none());
    }

    #[test]
    fn parses_answer() {
        let v = serde_json::json!({"choices":[{"message":{"content":"Hola."}}]});
        assert_eq!(extract_answer(&v).unwrap(), "Hola.");
        let v = serde_json::json!({"choices":[{"message":{"content":[{"type":"text","text":"A"},{"text":"B"}]}}]});
        assert_eq!(extract_answer(&v).unwrap(), "AB");
    }
}
