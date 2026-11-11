//! Voces neuronales gratuitas (servicio de lectura de Microsoft Edge).
//! Es un servicio no oficial: si deja de funcionar, ZENOS vuelve solo a la voz de Windows.

use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, http::HeaderValue, Message};

const TOKEN: &str = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";
const CHROME_VERSION: &str = "130.0.2849.68";

/// Una "persona" = voz neuronal + ajuste de ritmo y tono.
pub struct Persona {
    pub id: &'static str,
    pub label: &'static str,
    pub voice: &'static str,
    pub lang: &'static str,
    pub rate: i32,  // %
    pub pitch: i32, // Hz
}

pub const PERSONAS: &[Persona] = &[
    Persona { id: "dalia", label: "Dalia · cálida y natural (México)", voice: "es-MX-DaliaNeural", lang: "es-MX", rate: 0, pitch: 0 },
    Persona { id: "dalia-viva", label: "Dalia viva · animada y alegre (México)", voice: "es-MX-DaliaNeural", lang: "es-MX", rate: 12, pitch: 8 },
    Persona { id: "elvira", label: "Elvira · clara y profesional (España)", voice: "es-ES-ElviraNeural", lang: "es-ES", rate: 0, pitch: 0 },
    Persona { id: "elvira-serena", label: "Elvira serena · calmada (España)", voice: "es-ES-ElviraNeural", lang: "es-ES", rate: -8, pitch: -2 },
    Persona { id: "jorge", label: "Jorge · grave y seguro (México)", voice: "es-MX-JorgeNeural", lang: "es-MX", rate: 0, pitch: 0 },
    Persona { id: "alvaro", label: "Álvaro · firme y directo (España)", voice: "es-ES-AlvaroNeural", lang: "es-ES", rate: 3, pitch: 0 },
    Persona { id: "elena", label: "Elena · cercana (Argentina)", voice: "es-AR-ElenaNeural", lang: "es-AR", rate: 0, pitch: 0 },
    Persona { id: "salome", label: "Salomé · suave (Colombia)", voice: "es-CO-SalomeNeural", lang: "es-CO", rate: -3, pitch: 2 },
];

pub const DEFAULT_PERSONA: &str = "dalia";

pub fn persona(id: &str) -> &'static Persona {
    PERSONAS.iter().find(|p| p.id == id).unwrap_or(&PERSONAS[0])
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonaInfo {
    pub id: String,
    pub label: String,
    pub language: String,
}

pub fn list() -> Vec<PersonaInfo> {
    PERSONAS.iter().map(|p| PersonaInfo { id: format!("edge:{}", p.id), label: p.label.into(), language: p.lang.into() }).collect()
}

/// Token anti-abuso del servicio: SHA-256 del tiempo (en ticks de Windows, redondeado a 5 min) + token.
pub fn sec_ms_gec(now_unix_secs: u64) -> String {
    let mut ticks = now_unix_secs + 11_644_473_600;
    ticks -= ticks % 300;
    let ticks = ticks as u128 * 10_000_000;
    let mut h = Sha256::new();
    h.update(format!("{ticks}{TOKEN}").as_bytes());
    h.finalize().iter().map(|b| format!("{b:02X}")).collect()
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

pub fn build_ssml(text: &str, p: &Persona, rate_extra: i32) -> String {
    let rate = (p.rate + rate_extra).clamp(-50, 100);
    format!(
        "<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='{}'><voice name='{}'><prosody pitch='{:+}Hz' rate='{:+}%' volume='+0%'>{}</prosody></voice></speak>",
        p.lang,
        p.voice,
        p.pitch,
        rate,
        xml_escape(text)
    )
}

/// Extrae la parte de audio de un mensaje binario del servicio (2 bytes de longitud + cabeceras + datos).
pub fn audio_payload(frame: &[u8]) -> Option<&[u8]> {
    if frame.len() < 2 {
        return None;
    }
    let hlen = u16::from_be_bytes([frame[0], frame[1]]) as usize;
    let headers = frame.get(2..2 + hlen)?;
    if !String::from_utf8_lossy(headers).contains("Path:audio") {
        return None;
    }
    frame.get(2 + hlen..)
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn timestamp() -> String {
    // Formato aproximado que acepta el servicio.
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{secs}.{n}")
}

async fn fetch(text: &str, p: &Persona, rate_extra: i32) -> Result<Vec<u8>, String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let id = uuid::Uuid::new_v4().simple().to_string();
    let url = format!(
        "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1?TrustedClientToken={TOKEN}&ConnectionId={id}&Sec-MS-GEC={}&Sec-MS-GEC-Version=1-{CHROME_VERSION}",
        sec_ms_gec(now)
    );
    let mut req = url.into_client_request().map_err(|e| e.to_string())?;
    let h = req.headers_mut();
    let ua = format!("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36 Edg/{CHROME_VERSION}");
    for (k, v) in [
        ("Pragma", "no-cache"),
        ("Cache-Control", "no-cache"),
        ("Origin", "chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold"),
        ("Accept-Language", "en-US,en;q=0.9"),
        ("User-Agent", ua.as_str()),
    ] {
        h.insert(k, HeaderValue::from_str(v).map_err(|e| e.to_string())?);
    }
    let (mut ws, _) = tokio::time::timeout(Duration::from_secs(8), tokio_tungstenite::connect_async(req))
        .await
        .map_err(|_| "tiempo de conexión agotado".to_string())?
        .map_err(|e| format!("conexión: {e}"))?;

    let ts = timestamp();
    let config = format!(
        "X-Timestamp:{ts}\r\nContent-Type:application/json; charset=utf-8\r\nPath:speech.config\r\n\r\n{{\"context\":{{\"synthesis\":{{\"audio\":{{\"metadataoptions\":{{\"sentenceBoundaryEnabled\":\"false\",\"wordBoundaryEnabled\":\"false\"}},\"outputFormat\":\"riff-24khz-16bit-mono-pcm\"}}}}}}}}\r\n"
    );
    ws.send(Message::Text(config)).await.map_err(|e| e.to_string())?;
    let ssml = format!(
        "X-RequestId:{id}\r\nContent-Type:application/ssml+xml\r\nX-Timestamp:{ts}Z\r\nPath:ssml\r\n\r\n{}",
        build_ssml(text, p, rate_extra)
    );
    ws.send(Message::Text(ssml)).await.map_err(|e| e.to_string())?;

    let mut audio = Vec::new();
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(12), ws.next())
            .await
            .map_err(|_| "tiempo de respuesta agotado".to_string())?;
        match msg {
            Some(Ok(Message::Binary(b))) => {
                if let Some(data) = audio_payload(&b) {
                    audio.extend_from_slice(data);
                }
            }
            Some(Ok(Message::Text(t))) if t.contains("Path:turn.end") => break,
            Some(Ok(Message::Close(_))) | None => break,
            Some(Ok(_)) => {}
            Some(Err(e)) => return Err(e.to_string()),
        }
    }
    let _ = ws.close(None).await;
    if audio.len() < 1000 {
        return Err("el servicio no devolvió audio".into());
    }
    Ok(audio)
}

/// Sintetiza `text` y devuelve un WAV (24 kHz, 16 bits, mono). Bloqueante.
pub fn synthesize(text: &str, persona_id: &str, rate_extra: i32) -> Result<Vec<u8>, String> {
    let p = persona(persona_id);
    tauri::async_runtime::block_on(fetch(text, p, rate_extra))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gec_is_stable_within_five_minutes_and_hex() {
        let a = sec_ms_gec(1_700_000_000);
        let b = sec_ms_gec(1_700_000_000 + 10); // misma ventana de 5 minutos
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_lowercase()));
        assert_eq!(a, b);
        assert_ne!(sec_ms_gec(1_700_000_000), sec_ms_gec(1_700_000_000 + 600));
    }

    #[test]
    fn ssml_escapes_and_adjusts() {
        let s = build_ssml("Tom & <Jerry>", persona("dalia-viva"), 10);
        assert!(s.contains("Tom &amp; &lt;Jerry&gt;"));
        assert!(s.contains("rate='+22%'") && s.contains("pitch='+8Hz'") && s.contains("es-MX-DaliaNeural"));
        assert!(build_ssml("x", persona("elvira-serena"), 0).contains("rate='-8%'"));
    }

    #[test]
    fn parses_audio_frames() {
        let headers = b"X-RequestId:1\r\nContent-Type:audio/x-wav\r\nPath:audio\r\n";
        let mut f = (headers.len() as u16).to_be_bytes().to_vec();
        f.extend_from_slice(headers);
        f.extend_from_slice(&[1, 2, 3]);
        assert_eq!(audio_payload(&f), Some(&[1u8, 2, 3][..]));
        let other = b"Path:turn.start\r\n";
        let mut g = (other.len() as u16).to_be_bytes().to_vec();
        g.extend_from_slice(other);
        assert_eq!(audio_payload(&g), None);
        assert_eq!(audio_payload(&[0]), None);
    }

    #[test]
    fn unknown_persona_falls_back() {
        assert_eq!(persona("nope").id, DEFAULT_PERSONA);
    }
}
