//! Palabra de activación «Oye Zenos» (experimental).
//! Usa el reconocimiento de voz local de Windows con una gramática mínima, sin enviar audio a internet.

use std::sync::atomic::{AtomicU64, Ordering};
use tauri::AppHandle;

static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Activa o desactiva la escucha de la palabra de activación.
pub fn set_enabled(app: &AppHandle, enabled: bool) {
    let gen = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    if !enabled {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        if let Err(e) = imp::run(&app, gen, &GENERATION) {
            crate::log::write(&format!("palabra de activación: {e}"));
        }
    });
}

#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;
    use tauri::AppHandle;
    use windows::core::HSTRING;
    use windows::Foundation::TypedEventHandler;
    use windows::Media::SpeechRecognition::{
        SpeechContinuousRecognitionResultGeneratedEventArgs, SpeechContinuousRecognitionSession,
        SpeechRecognitionConfidence, SpeechRecognitionGrammarFileConstraint, SpeechRecognitionResultStatus,
        SpeechRecognizer,
    };
    use windows::Storage::StorageFile;
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

    const GRAMMAR: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<grammar version="1.0" xml:lang="es-ES" root="wake" xmlns="http://www.w3.org/2001/06/grammar" tag-format="semantics/1.0">
  <rule id="wake" scope="public">
    <one-of>
      <item>oye zenos</item>
      <item>hola zenos</item>
      <item>hey zenos</item>
      <item>ok zenos</item>
    </one-of>
  </rule>
</grammar>"#;

    pub fn run(app: &AppHandle, gen: u64, current: &'static AtomicU64) -> Result<(), String> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let path = std::env::temp_dir().join("zenos-wake.grxml");
        std::fs::write(&path, GRAMMAR).map_err(|e| e.to_string())?;
        let go = || -> windows::core::Result<(SpeechRecognizer, SpeechContinuousRecognitionSession)> {
            let file = StorageFile::GetFileFromPathAsync(&HSTRING::from(path.to_string_lossy().as_ref()))?.join()?;
            let rec = SpeechRecognizer::new()?;
            let constraint = SpeechRecognitionGrammarFileConstraint::Create(&file)?;
            rec.Constraints()?.Append(&constraint)?;
            let compiled = rec.CompileConstraintsAsync()?.join()?;
            if compiled.Status()? != SpeechRecognitionResultStatus::Success {
                return Err(windows::core::Error::new(
                    windows::core::HRESULT(-1),
                    "no se pudo preparar el reconocimiento de voz de Windows",
                ));
            }
            let session = rec.ContinuousRecognitionSession()?;
            let app2 = app.clone();
            session.ResultGenerated(&TypedEventHandler::new(
                move |_, args: windows::core::Ref<'_, SpeechContinuousRecognitionResultGeneratedEventArgs>| {
                    if let Some(args) = args.as_ref() {
                        let r = args.Result()?;
                        let ok = matches!(r.Confidence()?, SpeechRecognitionConfidence::High | SpeechRecognitionConfidence::Medium);
                        if ok && current.load(Ordering::SeqCst) == gen {
                            crate::log::write(&format!("palabra de activación: {}", r.Text()?));
                            crate::wake_trigger(&app2);
                        }
                    }
                    Ok(())
                },
            ))?;
            session.StartAsync()?.join()?;
            Ok((rec, session))
        };
        let (_rec, session) = go().map_err(|e| e.to_string())?;
        crate::log::write("palabra de activación: escuchando");
        while current.load(Ordering::SeqCst) == gen {
            std::thread::sleep(Duration::from_millis(500));
        }
        let _ = session.StopAsync().and_then(|a| a.join());
        Ok(())
    }
}

#[cfg(not(windows))]
mod imp {
    use std::sync::atomic::AtomicU64;
    use tauri::AppHandle;
    pub fn run(_: &AppHandle, _: u64, _: &'static AtomicU64) -> Result<(), String> {
        Err("solo está disponible en Windows".into())
    }
}
