//! Transcripción con Whisper en la nube (Groq u OpenAI).

use serde_json::Value;

/// Frases que Whisper "inventa" con silencio o ruido.
const HALLUCINATIONS: &[&str] = &[
    "subtítulos realizados por la comunidad de amara.org",
    "subtítulos por la comunidad de amara.org",
    "gracias por ver el video",
    "gracias por ver",
    "¡gracias por ver!",
    "suscríbete",
    "música",
    "thank you.",
    "thanks for watching!",
];

pub fn clean_transcript(text: &str) -> String {
    let t = text.trim();
    let norm = t.to_lowercase();
    let norm = norm.trim_matches(|c: char| c.is_ascii_punctuation() || c == '¡' || c == '¿' || c.is_whitespace());
    if norm.is_empty() || HALLUCINATIONS.iter().any(|h| h.trim_matches(|c: char| c.is_ascii_punctuation() || c == '¡') == norm) {
        return String::new();
    }
    t.to_string()
}

pub fn endpoint(stt_provider: &str) -> &'static str {
    match stt_provider {
        "openai" => "https://api.openai.com/v1/audio/transcriptions",
        _ => "https://api.groq.com/openai/v1/audio/transcriptions",
    }
}

pub fn display_name(stt_provider: &str) -> &'static str {
    match stt_provider {
        "openai" => "OpenAI",
        _ => "Groq",
    }
}

pub async fn transcribe(
    client: &reqwest::Client,
    stt_provider: &str,
    api_key: &str,
    model: &str,
    language: &str,
    wav: Vec<u8>,
) -> Result<String, String> {
    let name = display_name(stt_provider);
    let part = reqwest::multipart::Part::bytes(wav)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| e.to_string())?;
    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", model.to_string())
        .text("language", language.to_string())
        .text("response_format", "json")
        .text("temperature", "0");
    let resp = client
        .post(endpoint(stt_provider))
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| crate::ai::network_error(name, e))?;
    let status = resp.status().as_u16();
    let body = resp.text().await.map_err(|e| crate::ai::network_error(name, e))?;
    if !(200..300).contains(&status) {
        return Err(crate::ai::friendly_http_error(name, status, &body));
    }
    let v: Value = serde_json::from_str(&body).map_err(|_| "Respuesta inválida del servicio de voz.".to_string())?;
    Ok(clean_transcript(v["text"].as_str().unwrap_or_default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_hallucinations() {
        assert_eq!(clean_transcript(" Subtítulos realizados por la comunidad de Amara.org "), "");
        assert_eq!(clean_transcript("¡Gracias por ver!"), "");
        assert_eq!(clean_transcript("..."), "");
        assert_eq!(clean_transcript(" ¿Qué hora es? "), "¿Qué hora es?");
    }
}
