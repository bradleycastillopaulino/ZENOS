//! Adaptador para la API Messages de Anthropic (Claude).

use serde_json::{json, Value};

use super::{friendly_http_error, network_error, ChatRequest, ProviderInfo};

pub(crate) fn build_body(req: &ChatRequest<'_>) -> Value {
    let messages: Vec<Value> = req
        .messages
        .iter()
        .map(|m| json!({"role": m.role, "content": m.content}))
        .collect();
    json!({
        "model": req.model,
        "max_tokens": req.max_tokens,
        "system": req.system,
        "messages": messages,
    })
}

pub(crate) fn extract_answer(v: &Value) -> Option<String> {
    let blocks = v["content"].as_array()?;
    Some(
        blocks
            .iter()
            .filter(|b| b["type"] == "text")
            .filter_map(|b| b["text"].as_str())
            .collect::<Vec<_>>()
            .join(""),
    )
}

pub async fn chat(
    client: &reqwest::Client,
    provider: &ProviderInfo,
    api_key: &str,
    req: &ChatRequest<'_>,
) -> Result<String, String> {
    let resp = client
        .post(format!("{}/messages", provider.base_url))
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&build_body(req))
        .send()
        .await
        .map_err(|e| network_error(provider.name, e))?;
    let status = resp.status().as_u16();
    let text = resp.text().await.map_err(|e| network_error(provider.name, e))?;
    if !(200..300).contains(&status) {
        return Err(friendly_http_error(provider.name, status, &text));
    }
    let v: Value = serde_json::from_str(&text).map_err(|_| "Respuesta inválida de la IA.".to_string())?;
    extract_answer(&v).ok_or_else(|| "La IA devolvió una respuesta vacía.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::ChatMessage;

    #[test]
    fn body_and_parse() {
        let msgs = vec![ChatMessage { role: "user".into(), content: "hola".into() }];
        let req = ChatRequest { model: "claude-haiku-5-5", system: "sys", messages: &msgs, max_tokens: 50 };
        let b = build_body(&req);
        assert_eq!(b["system"], "sys");
        assert_eq!(b["messages"][0]["role"], "user");
        let v = serde_json::json!({"content":[{"type":"thinking","thinking":"x"},{"type":"text","text":"Hola"}]});
        assert_eq!(extract_answer(&v).unwrap(), "Hola");
    }
}
