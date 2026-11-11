//! Configuración local (JSON en la carpeta de datos de la aplicación).
//! Las claves API NUNCA se guardan aquí: ver `secrets.rs`.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::ai::provider_info;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppConfig {
    /// Proveedor de chat: openai | anthropic | gemini | groq | deepseek | openrouter
    pub provider: String,
    /// Modelo de chat. Vacío = modelo por defecto del proveedor.
    pub model: String,
    /// Servicio de transcripción: groq | openai
    pub stt_provider: String,
    /// Modelo de transcripción. Vacío = por defecto del servicio.
    pub stt_model: String,
    pub language: String,
    pub system_prompt: String,
    pub max_tokens: u32,
    /// Turnos de conversación recordados (pregunta + respuesta).
    pub history_turns: usize,
    pub hotkey: String,
    /// Nombre de la voz de Windows. Vacío = primera voz en español.
    pub voice: String,
    /// Velocidad de la voz (0.5 - 2.0).
    pub voice_rate: f64,
    /// Volumen (0.0 - 1.0).
    pub voice_volume: f64,
    /// Mantener la esfera visible también en reposo.
    pub orb_always_visible: bool,
    // Detección de voz
    pub silence_ms: u32,
    pub max_record_ms: u32,
    pub start_timeout_ms: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            provider: "anthropic".into(),
            model: String::new(),
            stt_provider: "groq".into(),
            stt_model: String::new(),
            language: "es".into(),
            system_prompt: "Eres AI Star, un asistente de voz. Responde siempre en español, de forma \
                natural, clara y breve (normalmente una a tres frases). Tu respuesta se leerá en voz \
                alta: no uses markdown, listas, emojis, tablas ni código."
                .into(),
            max_tokens: 1024,
            history_turns: 4,
            hotkey: "Ctrl+Alt+Space".into(),
            voice: String::new(),
            voice_rate: 1.0,
            voice_volume: 1.0,
            orb_always_visible: false,
            silence_ms: 1200,
            max_record_ms: 30_000,
            start_timeout_ms: 7_000,
        }
    }
}

impl AppConfig {
    pub fn effective_model(&self) -> String {
        if self.model.trim().is_empty() {
            provider_info(&self.provider)
                .map(|p| p.default_model.to_string())
                .unwrap_or_default()
        } else {
            self.model.trim().to_string()
        }
    }

    pub fn effective_stt_model(&self) -> String {
        if !self.stt_model.trim().is_empty() {
            return self.stt_model.trim().to_string();
        }
        match self.stt_provider.as_str() {
            "openai" => "gpt-4o-mini-transcribe".into(),
            _ => "whisper-large-v3-turbo".into(),
        }
    }

    /// Corrige valores fuera de rango para no romper el programa.
    pub fn sanitize(mut self) -> Self {
        let d = AppConfig::default();
        if provider_info(&self.provider).is_none() {
            self.provider = d.provider;
        }
        if !matches!(self.stt_provider.as_str(), "groq" | "openai") {
            self.stt_provider = d.stt_provider;
        }
        if self.hotkey.trim().is_empty() {
            self.hotkey = d.hotkey;
        }
        self.voice_rate = self.voice_rate.clamp(0.5, 2.0);
        self.voice_volume = self.voice_volume.clamp(0.0, 1.0);
        self.silence_ms = self.silence_ms.clamp(400, 5000);
        self.max_record_ms = self.max_record_ms.clamp(3000, 120_000);
        self.start_timeout_ms = self.start_timeout_ms.clamp(2000, 30_000);
        self.max_tokens = self.max_tokens.clamp(64, 8192);
        self.history_turns = self.history_turns.min(20);
        self
    }
}

pub fn config_path(dir: &Path) -> PathBuf {
    dir.join("config.json")
}

pub fn load(dir: &Path) -> AppConfig {
    let path = config_path(dir);
    match std::fs::read_to_string(&path) {
        Ok(s) => match serde_json::from_str::<AppConfig>(&s) {
            Ok(c) => c.sanitize(),
            Err(e) => {
                crate::log::write(&format!("config.json inválido ({e}); se usan valores por defecto"));
                AppConfig::default()
            }
        },
        Err(_) => AppConfig::default(),
    }
}

pub fn save(dir: &Path, cfg: &AppConfig) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(config_path(dir), json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_json_uses_defaults_and_sanitizes() {
        let c: AppConfig =
            serde_json::from_str(r#"{"provider":"nope","voiceRate":9,"sttProvider":"groq"}"#).unwrap();
        let c = c.sanitize();
        assert_eq!(c.provider, "anthropic");
        assert_eq!(c.voice_rate, 2.0);
        assert_eq!(c.hotkey, "Ctrl+Alt+Space");
    }

    #[test]
    fn default_models_exist_for_all_providers() {
        for p in crate::ai::PROVIDERS {
            let c = AppConfig { provider: p.id.into(), ..Default::default() };
            assert!(!c.effective_model().is_empty(), "{}", p.id);
        }
    }

    #[test]
    fn roundtrip_file() {
        let dir = std::env::temp_dir().join(format!("aistar-test-{}", std::process::id()));
        let mut c = AppConfig::default();
        c.provider = "groq".into();
        save(&dir, &c).unwrap();
        assert_eq!(load(&dir).provider, "groq");
        let _ = std::fs::remove_dir_all(dir);
    }
}
