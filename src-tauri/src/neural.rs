//! Voces neuronales gratuitas (servicio de lectura de Microsoft Edge).
//! Es un servicio no oficial: si deja de funcionar, ZENOS vuelve solo a la voz de Windows.

use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, http::HeaderValue, Message};

const TOKEN: &str = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";
const CHROME_VERSION: &str = "143.0.3650.75";

/// Una "persona" = voz neuronal + ajuste de ritmo y tono.
pub struct Persona {
    pub id: &'static str,
    pub label: &'static str,
    pub voice: &'static str,
    pub lang: &'static str,
    pub rate: i32,  // %
    pub pitch: i32, // Hz
    pub male: bool,
}

const fn p(id: &'static str, label: &'static str, voice: &'static str, lang: &'static str, rate: i32, pitch: i32, male: bool) -> Persona {
    Persona { id, label, voice, lang, rate, pitch, male }
}

pub const PERSONAS: &[Persona] = &[
    // Español
    p("dalia", "Dalia · cálida y natural (México)", "es-MX-DaliaNeural", "es-MX", 0, 0, false),
    p("dalia-viva", "Dalia viva · animada y alegre (México)", "es-MX-DaliaNeural", "es-MX", 12, 8, false),
    p("elvira", "Elvira · clara y profesional (España)", "es-ES-ElviraNeural", "es-ES", 0, 0, false),
    p("elvira-serena", "Elvira serena · calmada (España)", "es-ES-ElviraNeural", "es-ES", -8, -2, false),
    p("jorge", "Jorge · grave y seguro (México)", "es-MX-JorgeNeural", "es-MX", 0, 0, true),
    p("alvaro", "Álvaro · firme y directo (España)", "es-ES-AlvaroNeural", "es-ES", 3, 0, true),
    p("elena", "Elena · cercana (Argentina)", "es-AR-ElenaNeural", "es-AR", 0, 0, false),
    p("salome", "Salomé · suave (Colombia)", "es-CO-SalomeNeural", "es-CO", -3, 2, false),
    // English
    p("aria", "Aria · warm and natural (US)", "en-US-AriaNeural", "en-US", 0, 0, false),
    p("aria-lively", "Aria lively · upbeat (US)", "en-US-AriaNeural", "en-US", 12, 8, false),
    p("guy", "Guy · deep and steady (US)", "en-US-GuyNeural", "en-US", 0, 0, true),
    p("sonia", "Sonia · clear and polished (UK)", "en-GB-SoniaNeural", "en-GB", 0, 0, false),
    p("ryan", "Ryan · confident (UK)", "en-GB-RyanNeural", "en-GB", 3, 0, true),
    // Français
    p("denise", "Denise · chaleureuse et naturelle (France)", "fr-FR-DeniseNeural", "fr-FR", 0, 0, false),
    p("denise-vive", "Denise vive · enjouée (France)", "fr-FR-DeniseNeural", "fr-FR", 12, 8, false),
    p("henri", "Henri · grave et posé (France)", "fr-FR-HenriNeural", "fr-FR", 0, 0, true),
    p("eloise", "Éloïse · douce (France)", "fr-FR-EloiseNeural", "fr-FR", -3, 2, false),
    // Português
    p("francisca", "Francisca · calorosa e natural (Brasil)", "pt-BR-FranciscaNeural", "pt-BR", 0, 0, false),
    p("francisca-viva", "Francisca viva · animada (Brasil)", "pt-BR-FranciscaNeural", "pt-BR", 12, 8, false),
    p("antonio", "Antônio · grave e seguro (Brasil)", "pt-BR-AntonioNeural", "pt-BR", 0, 0, true),
    p("raquel", "Raquel · clara e serena (Portugal)", "pt-PT-RaquelNeural", "pt-PT", 0, 0, false),
    p("duarte", "Duarte · firme e direto (Portugal)", "pt-PT-DuarteNeural", "pt-PT", 3, 0, true),
];

pub const DEFAULT_PERSONA: &str = "dalia";

/// Primera voz del idioma ("es" | "en" | "fr" | "pt").
pub fn default_for(language: &str) -> &'static Persona {
    let lang = language.split(['-', '_']).next().unwrap_or("es").to_lowercase();
    PERSONAS.iter().find(|x| x.lang.starts_with(&lang)).unwrap_or(&PERSONAS[0])
}

pub fn persona(id: &str) -> &'static Persona {
    PERSONAS.iter().find(|p| p.id == id).unwrap_or(&PERSONAS[0])
}

pub fn find(id: &str) -> Option<&'static Persona> {
    PERSONAS.iter().find(|p| p.id == id)
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

static SKEW_SECS: AtomicI64 = AtomicI64::new(0);

fn unix_now() -> u64 {
    let raw = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    (raw + SKEW_SECS.load(Ordering::Relaxed)).max(0) as u64
}

/// Convierte una fecha HTTP ("Sun, 06 Nov 1994 08:49:37 GMT") en segundos Unix.
pub fn parse_http_date(s: &str) -> Option<i64> {
    let p: Vec<&str> = s.split_whitespace().collect();
    if p.len() < 5 {
        return None;
    }
    let day: i64 = p[1].parse().ok()?;
    let month = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
        .iter()
        .position(|m| m.eq_ignore_ascii_case(p[2]))? as i64
        + 1;
    let year: i64 = p[3].parse().ok()?;
    let t: Vec<i64> = p[4].split(':').filter_map(|x| x.parse().ok()).collect();
    if t.len() != 3 {
        return None;
    }
    // Días desde 1970 (algoritmo de Howard Hinnant).
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400 + t[0] * 3600 + t[1] * 60 + t[2])
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
    let id = uuid::Uuid::new_v4().simple().to_string();
    let url = format!(
        "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1?TrustedClientToken={TOKEN}&ConnectionId={id}&Sec-MS-GEC={}&Sec-MS-GEC-Version=2-{CHROME_VERSION}",
        crate::gec::token_v2(unix_now(), TOKEN)
    );
    let mut req = url.into_client_request().map_err(|e| e.to_string())?;
    let h = req.headers_mut();
    let ua = format!("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36 Edg/{CHROME_VERSION}");
    let muid = format!("muid={};", uuid::Uuid::new_v4().simple().to_string().to_uppercase());
    for (k, v) in [
        ("Pragma", "no-cache"),
        ("Cache-Control", "no-cache"),
        ("Origin", "chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold"),
        ("Accept-Language", "en-US,en;q=0.9"),
        ("User-Agent", ua.as_str()),
        ("Cookie", muid.as_str()),
    ] {
        h.insert(k, HeaderValue::from_str(v).map_err(|e| e.to_string())?);
    }
    let (mut ws, _) = tokio::time::timeout(Duration::from_secs(8), tokio_tungstenite::connect_async(req))
        .await
        .map_err(|_| "tiempo de conexión agotado".to_string())?
        .map_err(|e| {
            use tokio_tungstenite::tungstenite::Error as WsErr;
            if let WsErr::Http(resp) = &e {
                let date = resp.headers().get("date").and_then(|v| v.to_str().ok()).and_then(parse_http_date);
                if let Some(server) = date {
                    let local = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
                    SKEW_SECS.store(server - local, Ordering::Relaxed);
                }
                return format!("el servicio respondió {} (¿reloj o protocolo?)", resp.status());
            }
            format!("conexión: {e}")
        })?;

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
pub fn synthesize(text: &str, persona_id: &str, language: &str, rate_extra: i32) -> Result<Vec<u8>, String> {
    // Si la voz guardada es de otro idioma que el elegido, se usa la voz por defecto del idioma.
    let lang = language.split(['-', '_']).next().unwrap_or("es").to_lowercase();
    let p = match find(persona_id) {
        Some(x) if x.lang.starts_with(&lang) => x,
        _ => default_for(&lang),
    };
    tauri::async_runtime::block_on(async {
        match fetch(text, p, rate_extra).await {
            // Un 403 puede deberse al reloj: fetch ya guardó el desfase, se reintenta una vez.
            Err(e) if e.contains("403") => fetch(text, p, rate_extra).await,
            r => r,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_has_voices_and_unique_ids() {
        for l in ["es", "en", "fr", "pt"] {
            assert!(PERSONAS.iter().filter(|x| x.lang.starts_with(l)).count() >= 3, "{l}");
            assert!(default_for(l).lang.starts_with(l));
        }
        let mut ids: Vec<_> = PERSONAS.iter().map(|x| x.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), PERSONAS.len());
        assert_eq!(default_for("pt-BR").id, "francisca");
    }

    #[test]
    fn parses_http_dates() {
        assert_eq!(parse_http_date("Sun, 06 Nov 1994 08:49:37 GMT"), Some(784_111_777));
        assert_eq!(parse_http_date("Thu, 01 Jan 1970 00:00:00 GMT"), Some(0));
        assert_eq!(parse_http_date("basura"), None);
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
