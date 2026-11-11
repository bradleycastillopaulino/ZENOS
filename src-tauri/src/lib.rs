mod ai;
mod audio;
mod config;
mod log;
mod neural;
mod secrets;
mod stt;
mod text;
mod tools;
mod tts;
mod wake;

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
const CONFIRM: &str = "confirm";
const BUBBLE: &str = "bubble";
const TYPEBOX: &str = "typebox";
const BUBBLE_W: f64 = 380.0;
const BUBBLE_H: f64 = 230.0;
const TYPE_H: f64 = 96.0;
const ORB_BASE: f64 = 120.0;
const HISTORY_TTL: Duration = Duration::from_secs(300);

pub struct AppState {
    config: Mutex<AppConfig>,
    config_dir: PathBuf,
    http: reqwest::Client,
    history: Mutex<(Vec<ChatMessage>, Instant)>,
    record: Mutex<Option<Arc<RecordControl>>>,
    hotkey: Mutex<Option<Shortcut>>,
    listening: AtomicBool,
    speak_stop: Mutex<Arc<AtomicBool>>,
    pending_confirm: Mutex<Option<PendingConfirm>>,
}

struct PendingConfirm {
    text: String,
    danger: bool,
    tx: Option<tokio::sync::oneshot::Sender<bool>>,
}

fn escape_shortcut() -> Shortcut {
    Shortcut::new(None, Code::Escape)
}

// ------------------------------------------------------------------ ventana de la esfera

fn position_orb(win: &tauri::WebviewWindow, scale: f64, corner: &str) {
    let monitor = win.primary_monitor().ok().flatten().or_else(|| win.current_monitor().ok().flatten());
    let Some(m) = monitor else { return };
    let side = ORB_BASE * scale;
    let _ = win.set_size(tauri::LogicalSize::new(side, side));
    let Ok(size) = win.outer_size() else { return };
    let area = m.work_area();
    let margin = (16.0 * m.scale_factor()) as i32;
    let right = area.position.x + area.size.width as i32 - size.width as i32 - margin;
    let left = area.position.x + margin;
    let bottom = area.position.y + area.size.height as i32 - size.height as i32 - margin;
    let top = area.position.y + margin;
    let (x, y) = match corner {
        "bl" => (left, bottom),
        "tr" => (right, top),
        "tl" => (left, top),
        _ => (right, bottom),
    };
    let _ = win.set_position(PhysicalPosition::new(x, y));
}

fn apply_orb_layout(app: &AppHandle) {
    let (scale, corner) = {
        let st = app.state::<AppState>();
        let c = st.config.lock().unwrap();
        (c.orb_scale, c.orb_corner.clone())
    };
    if let Some(win) = app.get_webview_window(ORB) {
        position_orb(&win, scale, &corner);
    }
}

fn show_orb(app: &AppHandle) {
    apply_orb_layout(app);
    if let Some(win) = app.get_webview_window(ORB) {
        let _ = win.show();
        let _ = win.set_always_on_top(true);
        let _ = win.set_ignore_cursor_events(true);
    }
}

/// ¿El atajo abre el cuadro de escritura? (escritura activada, o no hay micrófono)
fn typing_enabled(app: &AppHandle) -> bool {
    app.state::<AppState>().config.lock().unwrap().type_mode || audio::input_names().is_empty()
}

fn trigger_assistant(app: &AppHandle) {
    log::write("atajo: activar asistente");
    if typing_enabled(app) {
        if app.get_webview_window(TYPEBOX).and_then(|w| w.is_visible().ok()).unwrap_or(false) {
            hide_typebox(app);
        } else {
            open_typebox(app);
        }
        return;
    }
    show_orb(app);
    let _ = app.emit_to(ORB, "zenos://hotkey", ());
}

// ------------------------------------------------------------------ recuadros sobre el orbe

fn build_overlay(app: &AppHandle, label: &str, w: f64, h: f64, focusable: bool) {
    let r = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title("ZENOS")
        .inner_size(w, h)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .focused(false)
        .focusable(focusable)
        .build();
    if let Err(e) = r {
        log::write(&format!("no se pudo crear la ventana {label}: {e}"));
    }
}

/// Coloca una ventana junto al orbe: encima en las esquinas de abajo, debajo en las de arriba.
fn place_near_orb(app: &AppHandle, label: &str, w: f64, h: f64, extra: f64) {
    let (Some(orb), Some(win)) = (app.get_webview_window(ORB), app.get_webview_window(label)) else { return };
    let corner = app.state::<AppState>().config.lock().unwrap().orb_corner.clone();
    let (Ok(op), Ok(os)) = (orb.outer_position(), orb.outer_size()) else { return };
    let sf = win.scale_factor().unwrap_or(1.0);
    let (wp, hp, gap, ex) = ((w * sf) as i32, (h * sf) as i32, (6.0 * sf) as i32, (extra * sf) as i32);
    let _ = win.set_size(tauri::PhysicalSize::new(wp as u32, hp as u32));
    let x = if corner.ends_with('r') { op.x + os.width as i32 - wp } else { op.x };
    let y = if corner.starts_with('b') { op.y - hp - gap - ex } else { op.y + os.height as i32 + gap + ex };
    let _ = win.set_position(PhysicalPosition::new(x.max(0), y.max(0)));
}

fn typebox_visible(app: &AppHandle) -> bool {
    app.get_webview_window(TYPEBOX).and_then(|w| w.is_visible().ok()).unwrap_or(false)
}

fn show_bubble(app: &AppHandle) {
    let extra = if typebox_visible(app) { TYPE_H + 6.0 } else { 0.0 };
    // El recuadro crece con el tamaño del texto para que no se recorte.
    let k = app.state::<AppState>().config.lock().unwrap().read_size.clamp(0.8, 2.0);
    place_near_orb(app, BUBBLE, BUBBLE_W * k, BUBBLE_H * k, extra);
    if let Some(w) = app.get_webview_window(BUBBLE) {
        let _ = w.show();
        let _ = w.set_ignore_cursor_events(true);
    }
}

fn hide_bubble_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(BUBBLE) {
        let _ = w.hide();
    }
}

fn open_typebox(app: &AppHandle) {
    place_near_orb(app, TYPEBOX, BUBBLE_W, TYPE_H, 0.0);
    if let Some(w) = app.get_webview_window(TYPEBOX) {
        let _ = w.show();
        let _ = w.set_focus();
    }
    let _ = app.emit_to(TYPEBOX, "zenos://typebox-open", ());
}

fn hide_typebox(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(TYPEBOX) {
        let _ = w.hide();
    }
}

/// Envía una línea al recuadro de lectura (si la lectura está activada).
fn bubble_emit(app: &AppHandle, kind: &str, text: &str) {
    if !app.state::<AppState>().config.lock().unwrap().read_along {
        return;
    }
    if kind != "end" {
        show_bubble(app);
    }
    let _ = app.emit_to(BUBBLE, "zenos://bubble", serde_json::json!({ "kind": kind, "text": text }));
}

fn open_settings(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(SETTINGS) {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    let result = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App("index.html".into()))
        .title("ZENOS")
        .inner_size(780.0, 600.0)
        .min_inner_size(700.0, 540.0)
        .center()
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
    if *shortcut == escape_shortcut() {
        if event_state == ShortcutState::Pressed {
            let _ = app.emit_to(ORB, "zenos://cancel", ());
        }
        return;
    }
    let is_main = app.state::<AppState>().hotkey.lock().unwrap().as_ref() == Some(shortcut);
    if !is_main {
        return;
    }
    match event_state {
        ShortcutState::Pressed => trigger_assistant(app),
        ShortcutState::Released => {
            let hold = app.state::<AppState>().config.lock().unwrap().hotkey_mode == "hold";
            if hold {
                let _ = app.emit_to(ORB, "zenos://hotkey-up", ());
            }
        }
    }
}

#[allow(dead_code)] // solo se usa en Windows
/// La palabra de activación solo inicia el asistente (nunca lo cancela).
pub(crate) fn wake_trigger(app: &AppHandle) {
    show_orb(app);
    let _ = app.emit_to(ORB, "zenos://wake", ());
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
    let wake_changed = state.config.lock().unwrap().wake_word != config.wake_word;
    *state.config.lock().unwrap() = config.clone();
    apply_orb_layout(&app);
    if wake_changed {
        wake::set_enabled(&app, config.wake_word);
    }
    let _ = app.emit("zenos://config", &config);
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
            "Falta la clave de {} para reconocer tu voz. Configúrala desde el icono de ZENOS.",
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
    let input_device = c.input_device.clone();
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        audio::record(vad_cfg, &input_device, control, |level| {
            let _ = app2.emit_to(ORB, "zenos://level", level);
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
            let _ = app.emit_to(ORB, "zenos://recorded", ());
            let text = stt::transcribe(&state.http, &c.stt_provider, &key, &c.effective_stt_model(), &c.language, wav).await?;
            log::write(&format!("transcripción: {}", brief(&text, 200)));
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

const MAX_STEPS: usize = 6;

/// Recorta texto para el registro (evita guardar contenido personal completo).
fn brief(s: &str, max: usize) -> String {
    let t: String = s.chars().take(max).collect();
    if s.chars().count() > max { format!("{t}…") } else { t }
}

fn open_confirm(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(CONFIRM) {
        let _ = w.close();
    }
    let r = WebviewWindowBuilder::new(app, CONFIRM, WebviewUrl::App("index.html".into()))
        .title("ZENOS · Confirmar")
        .inner_size(460.0, 300.0)
        .resizable(false)
        .minimizable(false)
        .maximizable(false)
        .always_on_top(true)
        .center()
        .build();
    match r {
        Ok(w) => {
            let _ = w.set_focus();
        }
        Err(e) => log::write(&format!("no se pudo abrir la confirmación: {e}")),
    }
}

/// Pide permiso al usuario con una ventana y espera hasta 60 s. Sin respuesta = no.
async fn confirm_action(app: &AppHandle, state: &State<'_, AppState>, a: &tools::Action) -> bool {
    let (tx, rx) = tokio::sync::oneshot::channel();
    *state.pending_confirm.lock().unwrap() =
        Some(PendingConfirm { text: tools::describe(a), danger: tools::is_dangerous(a), tx: Some(tx) });
    open_confirm(app);
    let allowed = matches!(tokio::time::timeout(Duration::from_secs(60), rx).await, Ok(Ok(true)));
    state.pending_confirm.lock().unwrap().take();
    if let Some(w) = app.get_webview_window(CONFIRM) {
        let _ = w.close();
    }
    allowed
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfirmInfo {
    text: String,
    danger: bool,
}

#[tauri::command]
fn get_confirm(state: State<'_, AppState>) -> Option<ConfirmInfo> {
    state.pending_confirm.lock().unwrap().as_ref().map(|p| ConfirmInfo { text: p.text.clone(), danger: p.danger })
}

#[tauri::command]
fn answer_confirm(state: State<'_, AppState>, allow: bool) {
    let tx = state.pending_confirm.lock().unwrap().as_mut().and_then(|p| p.tx.take());
    if let Some(tx) = tx {
        let _ = tx.send(allow);
    }
}

#[tauri::command]
async fn ask(app: AppHandle, state: State<'_, AppState>, text: String) -> Result<AskResult, String> {
    let c = cfg(&state);
    let provider = ai::provider_info(&c.provider).ok_or("Proveedor no válido.")?;
    let key = secrets::get(provider.id)
        .ok_or_else(|| format!("Falta la clave de {}. Configúrala desde el icono de ZENOS.", provider.name))?;

    bubble_emit(&app, "user", &text);
    let history = {
        let mut h = state.history.lock().unwrap();
        if h.1.elapsed() > HISTORY_TTL {
            h.0.clear();
        }
        h.0.clone()
    };
    let mut messages = history.clone();
    messages.push(ChatMessage { role: "user".into(), content: text.clone() });
    let model = c.effective_model();
    let system = if c.actions_enabled { format!("{}{}", c.system_prompt, tools::PROMPT) } else { c.system_prompt.clone() };

    let mut answer = String::new();
    let mut tainted = false; // ya se leyó contenido de archivos o carpetas en esta conversación
    for step in 0..MAX_STEPS {
        let raw = ai::chat(
            &state.http,
            provider,
            &key,
            ai::ChatRequest { model: &model, system: &system, messages: &messages, max_tokens: c.max_tokens },
        )
        .await?;
        log::write(&format!("respuesta ({}/{model}) paso {step}: {}", provider.id, brief(&raw, 300)));
        let (visible, action) = if c.actions_enabled { tools::parse_action(&raw) } else { (raw.clone(), None) };
        let Some(action) = action else {
            answer = if visible.is_empty() { "Hecho.".to_string() } else { visible };
            break;
        };
        let result = if tools::needs_confirm(&c.permission_level, &action, tainted) && !confirm_action(&app, &state, &action).await {
            log::write(&format!("acción cancelada por el usuario: {}", tools::describe(&action)));
            "El usuario NO autorizó esta acción. No la repitas; confirma que se canceló.".to_string()
        } else {
            log::write(&format!("acción: {}", tools::describe(&action)));
            let a2 = action.clone();
            match tauri::async_runtime::spawn_blocking(move || tools::run(&a2)).await.map_err(|e| e.to_string())? {
                Ok(r) => r,
                Err(e) => format!("ERROR: {e}"),
            }
        };
        if matches!(action.tool.as_str(), "leer" | "listar") {
            tainted = true;
            log::write(&format!("resultado: ({} caracteres de {})", result.chars().count(), action.tool));
        } else {
            log::write(&format!("resultado: {}", brief(&result, 200)));
        }
        messages.push(ChatMessage { role: "assistant".into(), content: raw });
        messages.push(ChatMessage { role: "user".into(), content: format!("RESULTADO DE LA ACCIÓN: {result}") });
    }
    if answer.is_empty() {
        answer = "No pude completar la tarea en pocos pasos.".into();
    }

    let mut kept = history;
    kept.push(ChatMessage { role: "user".into(), content: text });
    kept.push(ChatMessage { role: "assistant".into(), content: answer.clone() });
    let keep = c.history_turns * 2;
    let start = kept.len().saturating_sub(keep);
    *state.history.lock().unwrap() = (kept[start..].to_vec(), Instant::now());

    let chunks = text::speech_chunks(&text::clean_for_speech(&answer));
    Ok(AskResult { answer, chunks })
}

#[tauri::command]
fn stop_listen(state: State<'_, AppState>) {
    if let Some(c) = state.record.lock().unwrap().as_ref() {
        c.stop.store(true, Ordering::SeqCst);
    }
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

/// Sintetiza y reproduce por el altavoz elegido. Bloquea hasta terminar o hasta `stop_speaking`.
/// `level_to` = (ventana, evento) que recibe el nivel de la voz para animar.
async fn speak_inner(
    app: &AppHandle,
    state: &State<'_, AppState>,
    chunks: Vec<String>,
    device: Option<String>,
    level_to: (&'static str, &'static str),
) -> Result<(), String> {
    let c = cfg(state);
    let stop = Arc::new(AtomicBool::new(false));
    {
        let mut cur = state.speak_stop.lock().unwrap();
        cur.store(true, Ordering::SeqCst); // corta lo anterior
        *cur = stop.clone();
    }
    let device = device.unwrap_or_else(|| c.output_device.clone());
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let (tx, rx) = std::sync::mpsc::sync_channel::<(String, Result<Vec<u8>, String>)>(1);
        let stop_p = stop.clone();
        let (voice, language, rate) = (c.voice.clone(), c.language.clone(), c.voice_rate);
        std::thread::spawn(move || {
            for chunk in chunks {
                if stop_p.load(Ordering::SeqCst) {
                    break;
                }
                let r = tts::synthesize(&chunk, &voice, &language, rate);
                let failed = r.is_err();
                if tx.send((chunk, r)).is_err() || failed {
                    break;
                }
            }
        });
        for item in rx {
            if stop.load(Ordering::SeqCst) {
                break;
            }
            let (chunk_text, wav) = item;
            let wav = wav?;
            if level_to.0 == ORB {
                bubble_emit(&app2, "zenos", &chunk_text);
            }
            let (samples, hz) = audio::playback::parse_wav(&wav)?;
            audio::playback::play(&samples, hz, &device, c.voice_volume as f32, &stop, |level| {
                let _ = app2.emit_to(level_to.0, level_to.1, level);
            })
            .map_err(|e| {
                log::write(&e);
                e
            })?;
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn speak(app: AppHandle, state: State<'_, AppState>, chunks: Vec<String>) -> Result<(), String> {
    let r = speak_inner(&app, &state, chunks, None, (ORB, "zenos://level")).await;
    bubble_emit(&app, "end", "");
    r
}

/// El usuario envió texto desde el cuadro de escritura.
#[tauri::command]
fn submit_text(app: AppHandle, text: String) {
    hide_typebox(&app);
    if !text.trim().is_empty() {
        show_orb(&app);
        let _ = app.emit_to(ORB, "zenos://typed", text.trim().to_string());
    }
}

/// Desde el cuadro de escritura: hablar en vez de escribir.
#[tauri::command]
fn start_voice(app: AppHandle) {
    hide_typebox(&app);
    show_orb(&app);
    let _ = app.emit_to(ORB, "zenos://voice", ());
}

#[tauri::command]
fn close_typebox(app: AppHandle) {
    hide_typebox(&app);
}

#[tauri::command]
fn hide_bubble(app: AppHandle) {
    hide_bubble_window(&app);
}

#[tauri::command]
fn has_microphone() -> bool {
    !audio::input_names().is_empty()
}

#[tauri::command]
fn stop_speaking(state: State<'_, AppState>) {
    state.speak_stop.lock().unwrap().store(true, Ordering::SeqCst);
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Devices {
    inputs: Vec<String>,
    outputs: Vec<String>,
}

#[tauri::command]
async fn list_devices() -> Result<Devices, String> {
    tauri::async_runtime::spawn_blocking(|| Devices { inputs: audio::input_names(), outputs: audio::playback::output_names() })
        .await
        .map_err(|e| e.to_string())
}

/// Escucha 3 s y devuelve el nivel máximo (0..1). El nivel en vivo llega como `zenos://test-level`.
#[tauri::command]
async fn test_mic(app: AppHandle, device: String) -> Result<f32, String> {
    tauri::async_runtime::spawn_blocking(move || {
        audio::monitor(&device, 3000, |l| {
            let _ = app.emit_to(SETTINGS, "zenos://test-level", l);
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn test_speaker(app: AppHandle, state: State<'_, AppState>, device: String, voice: String) -> Result<(), String> {
    // Se prueba con la voz y el altavoz elegidos en pantalla, aunque aún no estén guardados.
    let saved = cfg(&state);
    if voice != saved.voice {
        state.config.lock().unwrap().voice = voice;
    }
    let r = speak_inner(&app, &state, vec!["Hola, soy Zenos. Si me escuchas, el altavoz funciona.".into()], Some(device), (SETTINGS, "zenos://test-level")).await;
    state.config.lock().unwrap().voice = saved.voice;
    r
}

/// Verifica la clave probando modelos conocidos hasta que uno responda; guarda clave y modelo.
#[tauri::command]
async fn setup_provider(state: State<'_, AppState>, provider: String, key: String) -> Result<String, String> {
    let info = ai::provider_info(&provider).ok_or("Proveedor no válido.")?;
    let key = key.trim().to_string();
    if key.is_empty() {
        return Err("Pega tu clave API.".into());
    }
    let mut candidates: Vec<&str> = vec![info.default_model];
    for m in info.suggested_models {
        if !candidates.contains(m) {
            candidates.push(m);
        }
    }
    let msgs = [ChatMessage { role: "user".into(), content: "Responde solo: hola".into() }];
    let mut chosen: Option<String> = None;
    let mut last_err = String::new();
    for m in candidates {
        match ai::chat(
            &state.http,
            info,
            &key,
            ai::ChatRequest { model: m, system: "Responde en español, muy breve.", messages: &msgs, max_tokens: 256 },
        )
        .await
        {
            Ok(_) => {
                chosen = Some(m.to_string());
                break;
            }
            Err(e) if e.contains("no existe") => last_err = e, // probar el siguiente modelo
            Err(e) => return Err(e),
        }
    }
    let model = chosen.ok_or(if last_err.is_empty() { "No se encontró un modelo disponible.".to_string() } else { last_err })?;
    secrets::set(info.id, &key)?;
    let mut c = state.config.lock().unwrap();
    c.provider = info.id.to_string();
    c.model = model.clone();
    if matches!(info.id, "groq" | "openai") {
        c.stt_provider = info.id.to_string();
    }
    config::save(&state.config_dir, &c)?;
    log::write(&format!("proveedor configurado: {} / {model}", info.id));
    Ok(model)
}

/// Verifica y guarda la clave del servicio de transcripción (Groq u OpenAI).
#[tauri::command]
async fn setup_stt(state: State<'_, AppState>, provider: String, key: String) -> Result<(), String> {
    if !matches!(provider.as_str(), "groq" | "openai") {
        return Err("Servicio de voz no válido.".into());
    }
    let key = key.trim().to_string();
    if key.is_empty() {
        return Err("Pega tu clave API.".into());
    }
    let name = stt::display_name(&provider);
    let url = if provider == "openai" { "https://api.openai.com/v1/models" } else { "https://api.groq.com/openai/v1/models" };
    let resp = state.http.get(url).bearer_auth(&key).send().await.map_err(|e| ai::network_error(name, e))?;
    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(ai::friendly_http_error(name, status, &body));
    }
    secrets::set(&provider, &key)?;
    let mut c = state.config.lock().unwrap();
    c.stt_provider = provider;
    config::save(&state.config_dir, &c)?;
    Ok(())
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

/// Abre una página web (solo https) en el navegador predeterminado.
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    if !url.starts_with("https://") || url.contains(char::is_whitespace) || url.contains('"') {
        return Err("Enlace no válido.".into());
    }
    #[cfg(windows)]
    let r = std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", &url]).spawn();
    #[cfg(not(windows))]
    let r = std::process::Command::new("xdg-open").arg(&url).spawn();
    r.map(|_| ()).map_err(|e| e.to_string())
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
    state.config_dir.join("logs").join("zenos.log").display().to_string()
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
    let _ = app.emit("zenos://config", &config);
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let hotkey = app.state::<AppState>().config.lock().unwrap().hotkey.clone();
    let talk = MenuItem::with_id(app, "talk", format!("Hablar ({hotkey})"), true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", "Mostrar / ocultar esfera", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Configuración…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Salir de ZENOS", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&talk, &toggle, &settings, &PredefinedMenuItem::separator(app)?, &quit])?;

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("ZENOS")
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
            log::write(&format!("ZENOS {} iniciando", app.package_info().version));

            let first_run = !config::config_path(&config_dir).exists();
            let config = config::load(&config_dir);
            if first_run {
                let _ = config::save(&config_dir, &config);
                let _ = app.autolaunch().enable();
            }

            let http = reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .connect_timeout(Duration::from_secs(10))
                .user_agent(concat!("ZENOS/", env!("CARGO_PKG_VERSION")))
                .build()?;

            app.manage(AppState {
                config: Mutex::new(config.clone()),
                config_dir,
                http,
                history: Mutex::new((Vec::new(), Instant::now())),
                record: Mutex::new(None),
                hotkey: Mutex::new(None),
                listening: AtomicBool::new(false),
                speak_stop: Mutex::new(Arc::new(AtomicBool::new(false))),
                pending_confirm: Mutex::new(None),
            });

            if let Err(e) = register_hotkey(&handle, &config.hotkey) {
                log::write(&e);
                // Si el atajo personalizado falla, intentar con el predeterminado.
                let _ = register_hotkey(&handle, &AppConfig::default().hotkey);
            }
            build_tray(&handle)?;
            build_overlay(&handle, BUBBLE, BUBBLE_W, BUBBLE_H, false);
            build_overlay(&handle, TYPEBOX, BUBBLE_W, TYPE_H, true);

            apply_orb_layout(&handle);
            if config.orb_always_visible {
                show_orb(&handle);
            }
            if config.wake_word {
                wake::set_enabled(&handle, true);
            }

            let autostarted = std::env::args().any(|a| a == "--autostart");
            let missing_keys = !secrets::has(&config.provider) || !secrets::has(&config.stt_provider);
            if first_run || !config.setup_done || (missing_keys && !autostarted) {
                open_settings(&handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // La esfera nunca se destruye: solo se oculta.
            if matches!(window.label(), ORB | BUBBLE | TYPEBOX) {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
            // Cerrar la ventana de confirmación sin responder = no autorizar.
            if window.label() == CONFIRM {
                if let WindowEvent::Destroyed = event {
                    let state = window.app_handle().state::<AppState>();
                    let tx = state.pending_confirm.lock().unwrap().as_mut().and_then(|p| p.tx.take());
                    if let Some(tx) = tx {
                        let _ = tx.send(false);
                    }
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
            submit_text,
            start_voice,
            close_typebox,
            hide_bubble,
            has_microphone,
            stop_listen,
            get_confirm,
            answer_confirm,
            test_provider,
            reset_conversation,
            speak,
            stop_speaking,
            list_devices,
            test_mic,
            test_speaker,
            setup_provider,
            setup_stt,
            list_voices,
            set_active,
            hide_orb,
            show_orb_cmd,
            open_settings_cmd,
            get_autostart,
            open_url,
            set_autostart,
            log_path,
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar ZENOS");
}
