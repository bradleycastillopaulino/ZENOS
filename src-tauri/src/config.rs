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
    /// Micrófono elegido (nombre exacto). Vacío = predeterminado del sistema.
    pub input_device: String,
    /// Altavoz elegido. Vacío = predeterminado del sistema.
    pub output_device: String,
    /// El asistente de configuración inicial ya se completó.
    pub setup_done: bool,
    /// "toggle" (pulsar para empezar/parar) | "hold" (mantener pulsado para hablar)
    pub hotkey_mode: String,
    /// Activar con la frase «Oye Zenos» (experimental, usa el reconocimiento de Windows).
    pub wake_word: bool,
    /// Tema de color de la esfera: menta | uva | mandarina | chicle | cielo
    pub theme: String,
    /// Tema de la interfaz: crema | lavanda | durazno (claros) · medianoche | carbon | bosque (oscuros)
    pub ui_theme: String,
    /// Avatar de la esfera: formas | gelatina | pulso | orbita
    pub avatar: String,
    /// Tamaño del texto del recuadro de lectura (0.8 - 2.0).
    pub read_size: f64,
    /// Escala de la esfera (0.7 - 1.8).
    pub orb_scale: f64,
    /// Esquina de la pantalla: br | bl | tr | tl
    pub orb_corner: String,
    /// El asistente puede abrir programas, manejar archivos y ejecutar comandos.
    /// Lectura: muestra en un recuadro sobre el orbe lo que dice ZENOS.
    pub read_along: bool,
    /// Escritura: el atajo abre un recuadro para escribirle. Se activa sola si no hay micrófono.
    pub type_mode: bool,
    pub actions_enabled: bool,
    /// "estricto" (confirma todo cambio) | "normal" (confirma borrar, sobrescribir y comandos) | "libre"
    pub permission_level: String,
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
            system_prompt: "Eres ZENOS, un asistente de voz. Responde siempre en español, de forma \
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
            input_device: String::new(),
            output_device: String::new(),
            setup_done: false,
            hotkey_mode: "toggle".into(),
            wake_word: false,
            theme: "menta".into(),
            ui_theme: "crema".into(),
            avatar: "formas".into(),
            read_size: 1.0,
            orb_scale: 1.0,
            orb_corner: "br".into(),
            read_along: false,
            type_mode: false,
            actions_enabled: true,
            permission_level: "normal".into(),
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
        if !matches!(self.hotkey_mode.as_str(), "toggle" | "hold") {
            self.hotkey_mode = d.hotkey_mode;
        }
        if !matches!(self.theme.as_str(), "menta" | "uva" | "mandarina" | "chicle" | "cielo") {
            self.theme = d.theme;
        }
        if !matches!(self.ui_theme.as_str(), "crema" | "lavanda" | "durazno" | "medianoche" | "carbon" | "bosque") {
            self.ui_theme = d.ui_theme;
        }
        if !matches!(self.avatar.as_str(), "formas" | "gelatina" | "pulso" | "orbita") {
            self.avatar = d.avatar;
        }
        if !matches!(self.orb_corner.as_str(), "br" | "bl" | "tr" | "tl") {
            self.orb_corner = d.orb_corner;
        }
        if !matches!(self.permission_level.as_str(), "estricto" | "normal" | "libre") {
            self.permission_level = d.permission_level;
        }
        self.orb_scale = self.orb_scale.clamp(0.7, 1.8);
        self.read_size = if self.read_size.is_finite() { self.read_size.clamp(0.8, 2.0) } else { 1.0 };
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
        let c: AppConfig = serde_json::from_str(r#"{"theme":"x","orbCorner":"zz","permissionLevel":"?","orbScale":9,"uiTheme":"neon","avatar":"robot","readSize":9}"#).unwrap();
        let c = c.sanitize();
        assert_eq!((c.theme.as_str(), c.orb_corner.as_str(), c.permission_level.as_str()), ("menta", "br", "normal"));
        assert_eq!((c.ui_theme.as_str(), c.read_size), ("crema", 2.0));
        assert_eq!(c.avatar, "formas");
        assert_eq!(c.orb_scale, 1.8);
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
        let dir = std::env::temp_dir().join(format!("zenos-test-{}", std::process::id()));
        let mut c = AppConfig::default();
        c.provider = "groq".into();
        save(&dir, &c).unwrap();
        assert_eq!(load(&dir).provider, "groq");
        let _ = std::fs::remove_dir_all(dir);
    }
}
