mod ai;
mod audio;
mod config;
mod log;
mod secrets;
mod stt;
mod text;
mod tts;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_autostart::ManagerExt as AutostartExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut, ShortcutState};

use ai::ChatMessage;
use audio::{vad::VadConfig, RecordControl, RecordOutcome};
use config::AppConfig;

const ORB: &str = "orb";
const SETTINGS: &str = "settings";
/// Mismos argumentos para todas las ventanas (WebView2 exige que coincidan).
/// Se permite reproducir audio sin interacción previa del usuario.
pub const BROWSER_ARGS: &str =
    "--autoplay-policy=no-user-gesture-required --disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection";
const HISTORY_TTL: Duration = Duration::from_secs(300);

pub struct AppState {
    config: Mutex<AppConfig>,
    config_dir: PathBuf,
    http: reqwest::Client,
    history: Mutex<(Vec<ChatMessage>, Instant)>,
    record: Mutex<Option<Arc<RecordControl>>>,
    hotkey: Mutex<Option<Shortcut>>,
    listening: AtomicBool,
}

fn escape_shortcut() -> Shortcut {
    Shortcut::new(None, Code::Escape)
}

// ------------------------------------------------------------------ ventana de la esfera

fn position_orb(win: &tauri::WebviewWindow) {
    let monitor = win.primary_monitor().ok().flatten().or_else(|| win.current_monitor().ok().flatten());
    let (Some(m), Ok(size)) = (monitor, win.outer_size()) else { return };
    let area = m.work_area();
    let margin = (16.0 * m.scale_factor()) as i32;
    let x = area.position.x + area.size.width as i32 - size.width as i32 - margin;
    let y = area.position.y + area.size.height as i32 - size.height as i32 - margin;
    let _ = win.set_position(PhysicalPosition::new(x, y));
}

fn show_orb(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(ORB) {
        position_orb(&win);
        let _ = win.show();
        let _ = win.set_always_on_top(true);
        let _ = win.set_ignore_cursor_events(true);
    }
}

fn trigger_assistant(app: &AppHandle) {
    log::write("atajo: activar asistente");
    show_orb(app);
    let _ = app.emit_to(ORB, "aistar://hotkey", ());
}

fn open_settings(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(SETTINGS) {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    let result = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App("index.html".into()))
        .title("AI Star · Configuración")
        .inner_size(460.0, 700.0)
        .min_inner_size(420.0, 560.0)
        .center()
        .additional_browser_args(BROWSER_ARGS)
        .build();
    if let Err(e) = result {
        log::write(&format!("no se pudo abrir configuración: {e}"));
    }
}

// ------------------------------------------------------------------ atajos

fn register_hotkey(app: &AppHandle, text: &str) -> Result<(), String> {
    let shortcut: Shortcut = text
        .parse()
        .map_err(|_| format!("Atajo no válido: «{text}». Ejemplo: Ctrl+Alt+Space"))?;
    let state = app.state::<AppState>();
    let mut current = state.hotkey.lock().unwrap();
    if current.as_ref() == Some(&shortcut) {
        return Ok(());
    }
    let gs = app.global_shortcut();
    gs.register(shortcut)
        .map_err(|e| format!("No se pudo registrar el atajo «{text}» (¿lo usa otra aplicación?): {e}"))?;
    if let Some(old) = current.take() {
        let _ = gs.unregister(old);
    }
    *current = Some(shortcut);
    log::write(&format!("atajo registrado: {text}"));
    Ok(())
}

fn on_shortcut(app: &AppHandle, shortcut: &Shortcut, event_state: ShortcutState) {
    if event_state != ShortcutState::Pressed {
        return;
    }
    if *shortcut == escape_shortcut() {
        let _ = app.emit_to(ORB, "aistar://cancel", ());
        return;
    }
    let is_main = app.state::<AppState>().hotkey.lock().unwrap().as_ref() == Some(shortcut);
    if is_main {
        trigger_assistant(app);
    }
}

// ------------------------------------------------------------------ comandos

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ListenResult {
    /// "text" | "nospeech" | "cancelled"
    kind: &'static str,
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AskResult {
    answer: String,
    chunks: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KeyStatus {
    provider: String,
    saved: bool,
}

fn cfg(state: &State<'_, AppState>) -> AppConfig {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> AppConfig {
    cfg(&state)
}

#[tauri::command]
fn get_providers() -> Vec<ai::ProviderInfo> {
    ai::PROVIDERS.to_vec()
}

#[tauri::command]
fn save_config(app: AppHandle, state: State<'_, AppState>, config: AppConfig) -> Result<AppConfig, String> {
    let config = config.sanitize();
    register_hotkey(&app, &config.hotkey)?;
    config::save(&state.config_dir, &config)?;
    *state.config.lock().unwrap() = config.clone();
    let _ = app.emit_to(ORB, "aistar://config", &config);
    Ok(config)
}

#[tauri::command]
fn key_status() -> Vec<KeyStatus> {
    ai::PROVIDERS
        .iter()
        .map(|p| KeyStatus { provider: p.id.to_string(), saved: secrets::has(p.id) })
        .collect()
}

#[tauri::command]
fn set_api_key(provider: String, key: String) -> Result<(), String> {
    if ai::provider_info(&provider).is_none() {
        return Err("Proveedor desconocido.".into());
    }
    secrets::set(&provider, &key)
}

#[tauri::command]
fn delete_api_key(provider: String) -> Result<(), String> {
    secrets::delete(&provider)
}

#[tauri::command]
async fn listen(app: AppHandle, state: State<'_, AppState>) -> Result<ListenResult, String> {
    let c = cfg(&state);
    let key = secrets::get(&c.stt_provider).ok_or_else(|| {
        format!(
            "Falta la clave de {} para reconocer tu voz. Configúrala desde el icono de AI Star.",
            stt::display_name(&c.stt_provider)
        )
    })?;
    if state.listening.swap(true, Ordering::SeqCst) {
        return Err("Ya estoy escuchando.".into());
    }
    let control = Arc::new(RecordControl::default());
    *state.record.lock().unwrap() = Some(control.clone());
    let vad_cfg = VadConfig {
        silence_ms: c.silence_ms,
        max_ms: c.max_record_ms,
        start_timeout_ms: c.start_timeout_ms,
        ..VadConfig::default()
    };
    let app2 = app.clone();
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        audio::record(vad_cfg, control, |level| {
            let _ = app2.emit_to(ORB, "aistar://level", level);
        })
    })
    .await;
    *state.record.lock().unwrap() = None;
    state.listening.store(false, Ordering::SeqCst);

    let outcome = outcome.map_err(|e| e.to_string())?.map_err(|e| {
        log::write(&e);
        e
    })?;
    match outcome {
        RecordOutcome::Cancelled => Ok(ListenResult { kind: "cancelled", text: String::new() }),
        RecordOutcome::NoSpeech => Ok(ListenResult { kind: "nospeech", text: String::new() }),
        RecordOutcome::Audio(wav) => {
            let _ = app.emit_to(ORB, "aistar://recorded", ());
            let text = stt::transcribe(&state.http, &c.stt_provider, &key, &c.effective_stt_model(), &c.language, wav).await?;
            log::write(&format!("transcripción: {text}"));
            if text.is_empty() {
                Ok(ListenResult { kind: "nospeech", text })
            } else {
                Ok(ListenResult { kind: "text", text })
            }
        }
    }
}

#[tauri::command]
fn cancel_listen(state: State<'_, AppState>) {
    if let Some(c) = state.record.lock().unwrap().as_ref() {
        c.cancel.store(true, Ordering::SeqCst);
    }
}

#[tauri::command]
async fn ask(state: State<'_, AppState>, text: String) -> Result<AskResult, String> {
    let c = cfg(&state);
    let provider = ai::provider_info(&c.provider).ok_or("Proveedor no válido.")?;
    let key = secrets::get(provider.id)
        .ok_or_else(|| format!("Falta la clave de {}. Configúrala desde el icono de AI Star.", provider.name))?;

    let mut messages = {
        let mut h = state.history.lock().unwrap();
        if h.1.elapsed() > HISTORY_TTL {
            h.0.clear();
        }
        h.0.clone()
    };
    messages.push(ChatMessage { role: "user".into(), content: text });
    let model = c.effective_model();
    let answer = ai::chat(
        &state.http,
        provider,
        &key,
        ai::ChatRequest { model: &model, system: &c.system_prompt, messages: &messages, max_tokens: c.max_tokens },
    )
    .await?;
    log::write(&format!("respuesta ({}/{model}): {answer}", provider.id));

    messages.push(ChatMessage { role: "assistant".into(), content: answer.clone() });
    let keep = c.history_turns * 2;
    let start = messages.len().saturating_sub(keep);
    *state.history.lock().unwrap() = (messages[start..].to_vec(), Instant::now());

    let chunks = text::speech_chunks(&text::clean_for_speech(&answer));
    Ok(AskResult { answer, chunks })
}

#[tauri::command]
async fn test_provider(state: State<'_, AppState>) -> Result<String, String> {
    let c = cfg(&state);
    let provider = ai::provider_info(&c.provider).ok_or("Proveedor no válido.")?;
    let key = secrets::get(provider.id).ok_or_else(|| format!("No hay clave guardada para {}.", provider.name))?;
    let model = c.effective_model();
    let msgs = [ChatMessage { role: "user".into(), content: "Responde solo: hola".into() }];
    ai::chat(
        &state.http,
        provider,
        &key,
        ai::ChatRequest { model: &model, system: "Responde en español, muy breve.", messages: &msgs, max_tokens: 256 },
    )
    .await
}

#[tauri::command]
fn reset_conversation(state: State<'_, AppState>) {
    state.history.lock().unwrap().0.clear();
}

#[tauri::command]
async fn synthesize(state: State<'_, AppState>, text: String) -> Result<tauri::ipc::Response, String> {
    let c = cfg(&state);
    let bytes = tauri::async_runtime::spawn_blocking(move || tts::synthesize(&text, &c.voice, &c.language, c.voice_rate))
        .await
        .map_err(|e| e.to_string())??;
    Ok(tauri::ipc::Response::new(bytes))
}

#[tauri::command]
async fn list_voices() -> Result<Vec<tts::Voice>, String> {
    tauri::async_runtime::spawn_blocking(tts::list).await.map_err(|e| e.to_string())?
}

/// La esfera avisa cuando hay actividad: mientras tanto Esc cancela.
#[tauri::command]
fn set_active(app: AppHandle, active: bool) {
    let gs = app.global_shortcut();
    let esc = escape_shortcut();
    if active {
        if !gs.is_registered(esc) {
            let _ = gs.register(esc);
        }
    } else if gs.is_registered(esc) {
        let _ = gs.unregister(esc);
    }
}

#[tauri::command]
fn hide_orb(app: AppHandle) {
    if let Some(w) = app.get_webview_window(ORB) {
        let _ = w.hide();
    }
}

#[tauri::command]
fn show_orb_cmd(app: AppHandle) {
    show_orb(&app);
}

#[tauri::command]
fn open_settings_cmd(app: AppHandle) {
    open_settings(&app);
}

#[tauri::command]
fn get_autostart(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let al = app.autolaunch();
    if enabled { al.enable() } else { al.disable() }.map_err(|e| e.to_string())
}

#[tauri::command]
fn log_path(state: State<'_, AppState>) -> String {
    state.config_dir.join("logs").join("ai-star.log").display().to_string()
}

// ------------------------------------------------------------------ bandeja

fn toggle_orb_pinned(app: &AppHandle) {
    let state = app.state::<AppState>();
    let config = {
        let mut c = state.config.lock().unwrap();
        c.orb_always_visible = !c.orb_always_visible;
        c.clone()
    };
    let _ = config::save(&state.config_dir, &config);
    if config.orb_always_visible {
        show_orb(app);
    }
    let _ = app.emit_to(ORB, "aistar://config", &config);
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let hotkey = app.state::<AppState>().config.lock().unwrap().hotkey.clone();
    let talk = MenuItem::with_id(app, "talk", format!("Hablar ({hotkey})"), true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", "Mostrar / ocultar esfera", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Configuración…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Salir de AI Star", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&talk, &toggle, &settings, &PredefinedMenuItem::separator(app)?, &quit])?;

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("AI Star")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "talk" => trigger_assistant(app),
            "toggle" => toggle_orb_pinned(app),
            "settings" => open_settings(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                open_settings(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

// ------------------------------------------------------------------ arranque

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Abrir la app otra vez muestra la configuración.
            open_settings(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| on_shortcut(app, shortcut, event.state))
                .build(),
        )
        .setup(|app| {
            let handle = app.handle().clone();
            let config_dir = app.path().app_config_dir()?;
            log::init(config_dir.join("logs"));
            log::write(&format!("AI Star {} iniciando", app.package_info().version));

            let first_run = !config::config_path(&config_dir).exists();
            let config = config::load(&config_dir);
            if first_run {
                let _ = config::save(&config_dir, &config);
                let _ = app.autolaunch().enable();
            }

            let http = reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .connect_timeout(Duration::from_secs(10))
                .user_agent(concat!("AI-Star/", env!("CARGO_PKG_VERSION")))
                .build()?;

            app.manage(AppState {
                config: Mutex::new(config.clone()),
                config_dir,
                http,
                history: Mutex::new((Vec::new(), Instant::now())),
                record: Mutex::new(None),
                hotkey: Mutex::new(None),
                listening: AtomicBool::new(false),
            });

            if let Err(e) = register_hotkey(&handle, &config.hotkey) {
                log::write(&e);
                // Si el atajo personalizado falla, intentar con el predeterminado.
                let _ = register_hotkey(&handle, &AppConfig::default().hotkey);
            }
            build_tray(&handle)?;

            if let Some(orb) = app.get_webview_window(ORB) {
                position_orb(&orb);
                if config.orb_always_visible {
                    show_orb(&handle);
                }
            }

            let autostarted = std::env::args().any(|a| a == "--autostart");
            let missing_keys = !secrets::has(&config.provider) || !secrets::has(&config.stt_provider);
            if first_run || (missing_keys && !autostarted) {
                open_settings(&handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // La esfera nunca se destruye: solo se oculta.
            if window.label() == ORB {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            get_providers,
            save_config,
            key_status,
            set_api_key,
            delete_api_key,
            listen,
            cancel_listen,
            ask,
            test_provider,
            reset_conversation,
            synthesize,
            list_voices,
            set_active,
            hide_orb,
            show_orb_cmd,
            open_settings_cmd,
            get_autostart,
            set_autostart,
            log_path,
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar AI Star");
}
