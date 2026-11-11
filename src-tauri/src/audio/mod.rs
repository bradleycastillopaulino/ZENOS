pub mod dsp;
pub mod playback;
pub mod vad;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use dsp::{encode_wav, to_mono, Resampler};
use vad::{rms, Vad, VadConfig, VadState, FRAME, SAMPLE_RATE};

pub enum RecordOutcome {
    /// WAV 16 kHz mono listo para transcribir.
    Audio(Vec<u8>),
    NoSpeech,
    Cancelled,
}

/// Controles compartidos de la grabación en curso.
#[derive(Default)]
pub struct RecordControl {
    pub cancel: AtomicBool,
    pub stop: AtomicBool,
}

/// Micrófono abierto: mantiene vivo el stream y entrega bloques mono.
struct Input {
    _stream: cpal::Stream,
    rx: mpsc::Receiver<Vec<f32>>,
    rate: u32,
}

pub fn input_names() -> Vec<String> {
    cpal::default_host()
        .input_devices()
        .map(|it| it.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}

fn open_input(name: &str) -> Result<Input, String> {
    let host = cpal::default_host();
    let mut device = None;
    if !name.is_empty() {
        device = host
            .input_devices()
            .ok()
            .and_then(|mut it| it.find(|d| d.name().map(|n| n == name).unwrap_or(false)));
        if device.is_none() {
            crate::log::write(&format!("micrófono «{name}» no encontrado; se usa el predeterminado"));
        }
    }
    let device = device
        .or_else(|| host.default_input_device())
        .ok_or("No se encontró ningún micrófono.")?;
    let supported = device
        .default_input_config()
        .map_err(|e| format!("No se pudo abrir el micrófono: {e}"))?;
    let channels = supported.channels() as usize;
    let rate = supported.sample_rate().0;
    let format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();

    let (tx, rx) = mpsc::channel::<Vec<f32>>();
    let err_fn = |e| crate::log::write(&format!("error de audio: {e}"));
    let stream = match format {
        cpal::SampleFormat::F32 => {
            device.build_input_stream(&config, move |d: &[f32], _: &_| { let _ = tx.send(to_mono(d, channels)); }, err_fn, None)
        }
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config,
            move |d: &[i16], _: &_| {
                let f: Vec<f32> = d.iter().map(|s| *s as f32 / 32768.0).collect();
                let _ = tx.send(to_mono(&f, channels));
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::U16 => device.build_input_stream(
            &config,
            move |d: &[u16], _: &_| {
                let f: Vec<f32> = d.iter().map(|s| (*s as f32 - 32768.0) / 32768.0).collect();
                let _ = tx.send(to_mono(&f, channels));
            },
            err_fn,
            None,
        ),
        other => return Err(format!("Formato de micrófono no soportado: {other:?}")),
    }
    .map_err(|e| format!("No se pudo abrir el micrófono: {e}"))?;
    stream.play().map_err(|e| format!("No se pudo iniciar el micrófono: {e}"))?;
    Ok(Input { _stream: stream, rx, rate })
}

/// Escucha unos segundos y devuelve el nivel máximo (0..1). Sirve para probar el micrófono.
pub fn monitor(device: &str, millis: u64, mut on_level: impl FnMut(f32)) -> Result<f32, String> {
    let input = open_input(device)?;
    let end = std::time::Instant::now() + Duration::from_millis(millis);
    let (mut peak, mut smoothed) = (0.0f32, 0.0f32);
    while std::time::Instant::now() < end {
        match input.rx.recv_timeout(Duration::from_millis(100)) {
            Ok(chunk) => {
                let target = (rms(&chunk) * 9.0).sqrt().min(1.0);
                smoothed = if target > smoothed { target } else { smoothed * 0.8 + target * 0.2 };
                peak = peak.max(smoothed);
                on_level(smoothed);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return Err("El micrófono se desconectó.".into()),
        }
    }
    Ok(peak)
}

/// Graba hasta detectar el final de la frase.
/// `on_level` recibe el nivel (0..1) unas 30 veces por segundo.
/// Bloqueante: llamar desde un hilo dedicado.
pub fn record(
    cfg: VadConfig,
    device: &str,
    control: Arc<RecordControl>,
    mut on_level: impl FnMut(f32),
) -> Result<RecordOutcome, String> {
    let input = open_input(device)?;
    let mut resampler = Resampler::new(input.rate, SAMPLE_RATE);
    let mut vad = Vad::new(cfg);
    let mut pending: Vec<f32> = Vec::with_capacity(FRAME * 4);
    let mut smoothed = 0.0f32;

    let outcome = loop {
        if control.cancel.load(Ordering::SeqCst) {
            break RecordOutcome::Cancelled;
        }
        if control.stop.load(Ordering::SeqCst) {
            break match vad.finish() {
                VadState::Done => RecordOutcome::Audio(encode_wav(&vad.audio, SAMPLE_RATE)),
                _ => RecordOutcome::NoSpeech,
            };
        }
        let chunk = match input.rx.recv_timeout(Duration::from_millis(100)) {
            Ok(c) => c,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("El micrófono se desconectó.".into());
            }
        };
        pending.extend(resampler.process(&chunk));
        let mut state = VadState::Continue;
        let mut peak = 0.0f32;
        while pending.len() >= FRAME {
            let frame: Vec<f32> = pending.drain(..FRAME).collect();
            peak = peak.max(rms(&frame));
            state = vad.push(&frame);
            if state != VadState::Continue {
                break;
            }
        }
        // Nivel perceptual 0..1 suavizado para la animación.
        let target = (peak * 9.0).sqrt().min(1.0);
        smoothed = if target > smoothed { target } else { smoothed * 0.8 + target * 0.2 };
        on_level(smoothed);
        match state {
            VadState::Continue => {}
            VadState::Done => break RecordOutcome::Audio(encode_wav(&vad.audio, SAMPLE_RATE)),
            VadState::NoSpeech => break RecordOutcome::NoSpeech,
        }
    };
    Ok(outcome)
}
