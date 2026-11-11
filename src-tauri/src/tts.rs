//! Síntesis de voz con el motor nativo de Windows (Windows.Media.SpeechSynthesis).
//! Devuelve un WAV que el frontend reproduce y analiza para animar la esfera.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Voice {
    pub name: String,
    pub language: String,
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
                Some(Voice {
                    name: v.DisplayName().ok()?.to_string(),
                    language: v.Language().ok()?.to_string(),
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

pub use imp::{list, synthesize};

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn windows_lists_voices_and_synthesizes_wav() {
        let voices = super::list().expect("listar voces");
        assert!(!voices.is_empty(), "Windows debe tener al menos una voz");
        let wav = super::synthesize("Hola, soy ZENOS.", "", "es", 1.0).expect("sintetizar");
        assert_eq!(&wav[0..4], b"RIFF");
        assert!(wav.len() > 4000, "audio demasiado corto: {}", wav.len());
    }
}
