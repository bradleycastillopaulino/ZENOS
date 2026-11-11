//! Síntesis de voz: voces neuronales gratuitas (`neural`) con respaldo al motor de Windows
//! (Windows.Media.SpeechSynthesis). Siempre devuelve un WAV que reproduce `audio::playback`.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Voice {
    /// "edge:<persona>" (neuronal) o "win:<nombre>" (voz de Windows).
    pub id: String,
    pub label: String,
    pub language: String,
    pub kind: &'static str,
}

#[cfg(windows)]
mod imp {
    use super::Voice;
    use windows::core::HSTRING;
    use windows::Media::SpeechSynthesis::{SpeechSynthesizer, VoiceInformation};
    use windows::Storage::Streams::DataReader;
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

    fn init_com() {
        // Puede devolver S_FALSE / RPC_E_CHANGED_MODE si ya está inicializado: se ignora.
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
    }

    fn all() -> windows::core::Result<Vec<VoiceInformation>> {
        Ok(SpeechSynthesizer::AllVoices()?.into_iter().collect())
    }

    pub fn list() -> Result<Vec<Voice>, String> {
        init_com();
        let voices = all().map_err(|e| e.to_string())?;
        Ok(voices
            .iter()
            .filter_map(|v| {
                let name = v.DisplayName().ok()?.to_string();
                Some(Voice {
                    id: format!("win:{name}"),
                    label: format!("{name} (voz de Windows)"),
                    language: v.Language().ok()?.to_string(),
                    kind: "windows",
                })
            })
            .collect())
    }

    fn pick(preferred: &str, language: &str) -> windows::core::Result<Option<VoiceInformation>> {
        let voices = all()?;
        if !preferred.is_empty() {
            if let Some(v) = voices.iter().find(|v| v.DisplayName().map(|n| n.to_string() == preferred).unwrap_or(false)) {
                return Ok(Some(v.clone()));
            }
        }
        let lang = language.to_lowercase();
        Ok(voices
            .into_iter()
            .find(|v| v.Language().map(|l| l.to_string().to_lowercase().starts_with(&lang)).unwrap_or(false)))
    }

    pub fn synthesize(text: &str, voice: &str, language: &str, rate: f64) -> Result<Vec<u8>, String> {
        init_com();
        let run = || -> windows::core::Result<Vec<u8>> {
            let synth = SpeechSynthesizer::new()?;
            if let Some(v) = pick(voice, language)? {
                synth.SetVoice(&v)?;
            }
            synth.Options()?.SetSpeakingRate(rate)?;
            let stream = synth.SynthesizeTextToStreamAsync(&HSTRING::from(text))?.join()?;
            let size = stream.Size()? as u32;
            let input = stream.GetInputStreamAt(0)?;
            let reader = DataReader::CreateDataReader(&input)?;
            reader.LoadAsync(size)?.join()?;
            let mut buf = vec![0u8; size as usize];
            reader.ReadBytes(&mut buf)?;
            Ok(buf)
        };
        run().map_err(|e| {
            crate::log::write(&format!("TTS: {e}"));
            "No se pudo generar la voz.".to_string()
        })
    }
}

#[cfg(not(windows))]
mod imp {
    use super::Voice;
    pub fn list() -> Result<Vec<Voice>, String> {
        Ok(vec![])
    }
    pub fn synthesize(_: &str, _: &str, _: &str, _: f64) -> Result<Vec<u8>, String> {
        Err("La voz solo está disponible en Windows.".into())
    }
}

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Instante (s) hasta el que se evita el servicio neuronal tras un fallo.
static NEURAL_DOWN_UNTIL: AtomicU64 = AtomicU64::new(0);

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Todas las voces: neuronales primero y luego las de Windows.
pub fn list() -> Result<Vec<Voice>, String> {
    let mut out: Vec<Voice> = crate::neural::list()
        .into_iter()
        .map(|p| Voice { id: p.id, label: p.label, language: p.language, kind: "neural" })
        .collect();
    out.extend(imp::list().unwrap_or_default());
    Ok(out)
}

/// `voice`: "" o "edge:<persona>" → neuronal con respaldo a Windows; "win:<nombre>" o un nombre antiguo → Windows.
pub fn synthesize(text: &str, voice: &str, language: &str, rate: f64) -> Result<Vec<u8>, String> {
    let (windows_name, neural_id) = if let Some(n) = voice.strip_prefix("win:") {
        (n.to_string(), None)
    } else if let Some(id) = voice.strip_prefix("edge:") {
        (String::new(), Some(id.to_string()))
    } else if voice.is_empty() {
        (String::new(), Some(crate::neural::DEFAULT_PERSONA.to_string()))
    } else {
        (voice.to_string(), None) // configuración antigua: nombre de voz de Windows
    };
    if let Some(id) = neural_id {
        if now() >= NEURAL_DOWN_UNTIL.load(Ordering::Relaxed) {
            let extra = ((rate - 1.0) * 100.0).round() as i32;
            match crate::neural::synthesize(text, &id, extra) {
                Ok(wav) => return Ok(wav),
                Err(e) => {
                    crate::log::write(&format!("voz neuronal no disponible ({e}); se usa la voz de Windows"));
                    NEURAL_DOWN_UNTIL.store(now() + 300, Ordering::Relaxed);
                }
            }
        }
    }
    imp::synthesize(text, &windows_name, language, rate)
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn windows_lists_voices_and_synthesizes_wav() {
        let voices = super::list().expect("listar voces");
        assert!(!voices.is_empty(), "Windows debe tener al menos una voz");
        let wav = super::synthesize("Hola, soy ZENOS.", "win:", "es", 1.0).expect("sintetizar");
        assert_eq!(&wav[0..4], b"RIFF");
        assert!(wav.len() > 4000, "audio demasiado corto: {}", wav.len());
    }
}
