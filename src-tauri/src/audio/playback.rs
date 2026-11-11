//! Reproducción de audio por el altavoz elegido (cpal), con nivel para animar la esfera.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use super::dsp::{to_mono, Resampler};

/// Decodifica un WAV PCM 16 bits o float 32 bits. Devuelve audio mono y la frecuencia.
pub fn parse_wav(bytes: &[u8]) -> Result<(Vec<f32>, u32), String> {
    let bad = || "El audio de la voz no es válido.".to_string();
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(bad());
    }
    let mut pos = 12usize;
    let mut fmt: Option<(u16, u16, u32, u16)> = None; // tag, canales, hz, bits
    let mut data: Option<&[u8]> = None;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let start = pos + 8;
        let end = start.saturating_add(size).min(bytes.len());
        if id == b"fmt " && end - start >= 16 {
            let b = &bytes[start..end];
            fmt = Some((
                u16::from_le_bytes([b[0], b[1]]),
                u16::from_le_bytes([b[2], b[3]]),
                u32::from_le_bytes(b[4..8].try_into().unwrap()),
                u16::from_le_bytes([b[14], b[15]]),
            ));
        } else if id == b"data" {
            data = Some(&bytes[start..end]);
            break;
        }
        pos = start.saturating_add(size).saturating_add(size & 1);
    }
    let ((tag, channels, hz, bits), data) = fmt.zip(data).ok_or_else(bad)?;
    let interleaved: Vec<f32> = match (tag, bits) {
        (1 | 0xFFFE, 16) => data
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0)
            .collect(),
        (3 | 0xFFFE, 32) => data
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect(),
        _ => return Err(bad()),
    };
    if hz == 0 || channels == 0 {
        return Err(bad());
    }
    Ok((to_mono(&interleaved, channels as usize), hz))
}

pub fn output_names() -> Vec<String> {
    cpal::default_host()
        .output_devices()
        .map(|it| it.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}

fn pick_output(host: &cpal::Host, name: &str) -> Result<cpal::Device, String> {
    if !name.is_empty() {
        if let Ok(mut devices) = host.output_devices() {
            if let Some(d) = devices.find(|d| d.name().map(|n| n == name).unwrap_or(false)) {
                return Ok(d);
            }
        }
        crate::log::write(&format!("altavoz «{name}» no encontrado; se usa el predeterminado"));
    }
    host.default_output_device().ok_or_else(|| "No se encontró ningún altavoz.".to_string())
}

fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    data: Arc<Vec<f32>>,
    pos: Arc<AtomicUsize>,
    level: Arc<AtomicU32>,
    volume: f32,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    device.build_output_stream(
        config,
        move |out: &mut [T], _: &cpal::OutputCallbackInfo| {
            let mut p = pos.load(Ordering::Relaxed);
            let mut sum = 0.0f32;
            let mut n = 0usize;
            for frame in out.chunks_mut(channels) {
                let raw = if p < data.len() {
                    let s = data[p];
                    p += 1;
                    s
                } else {
                    0.0
                };
                sum += raw * raw;
                n += 1;
                let v = T::from_sample((raw * volume).clamp(-1.0, 1.0));
                for s in frame.iter_mut() {
                    *s = v;
                }
            }
            pos.store(p, Ordering::Relaxed);
            let rms = if n > 0 { (sum / n as f32).sqrt() } else { 0.0 };
            level.store(rms.to_bits(), Ordering::Relaxed);
        },
        |e| crate::log::write(&format!("error de altavoz: {e}")),
        None,
    )
}

/// Reproduce y bloquea hasta terminar o hasta que `stop` sea true.
/// `on_level` recibe el nivel 0..1 de la voz unas 30 veces por segundo.
pub fn play(
    samples: &[f32],
    rate: u32,
    device_name: &str,
    volume: f32,
    stop: &AtomicBool,
    mut on_level: impl FnMut(f32),
) -> Result<(), String> {
    let host = cpal::default_host();
    let device = pick_output(&host, device_name)?;
    let supported = device
        .default_output_config()
        .map_err(|e| format!("No se pudo abrir el altavoz: {e}"))?;
    let channels = supported.channels() as usize;
    let out_rate = supported.sample_rate().0;
    let format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();

    let data = Arc::new(Resampler::new(rate, out_rate).process(samples));
    let total = data.len();
    let pos = Arc::new(AtomicUsize::new(0));
    let level = Arc::new(AtomicU32::new(0));

    let stream = match format {
        cpal::SampleFormat::F32 => build::<f32>(&device, &config, channels, data.clone(), pos.clone(), level.clone(), volume),
        cpal::SampleFormat::I16 => build::<i16>(&device, &config, channels, data.clone(), pos.clone(), level.clone(), volume),
        cpal::SampleFormat::U16 => build::<u16>(&device, &config, channels, data.clone(), pos.clone(), level.clone(), volume),
        other => return Err(format!("Formato de altavoz no soportado: {other:?}")),
    }
    .map_err(|e| format!("No se pudo abrir el altavoz: {e}"))?;
    stream.play().map_err(|e| format!("No se pudo reproducir: {e}"))?;

    let mut smoothed = 0.0f32;
    while pos.load(Ordering::Relaxed) < total && !stop.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(33));
        let rms = f32::from_bits(level.load(Ordering::Relaxed));
        let target = (rms * 5.0).min(1.0);
        smoothed = if target > smoothed { target } else { smoothed * 0.75 + target * 0.25 };
        on_level(smoothed);
    }
    if !stop.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(150)); // vaciar el búfer del dispositivo
    }
    on_level(0.0);
    drop(stream);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::dsp::encode_wav;

    #[test]
    fn wav_roundtrip() {
        let src = vec![0.0, 0.5, -0.5, 1.0];
        let (out, hz) = parse_wav(&encode_wav(&src, 22_050)).unwrap();
        assert_eq!(hz, 22_050);
        assert_eq!(out.len(), 4);
        assert!((out[1] - 0.5).abs() < 0.001 && (out[2] + 0.5).abs() < 0.001);
    }

    #[test]
    fn skips_extra_chunks_and_downmixes_stereo() {
        let mut w = Vec::new();
        w.extend_from_slice(b"RIFF");
        w.extend_from_slice(&0u32.to_le_bytes());
        w.extend_from_slice(b"WAVE");
        w.extend_from_slice(b"LIST");
        w.extend_from_slice(&3u32.to_le_bytes());
        w.extend_from_slice(&[1, 2, 3, 0]); // relleno de longitud impar
        w.extend_from_slice(b"fmt ");
        w.extend_from_slice(&16u32.to_le_bytes());
        for v in [1u16, 2] {
            w.extend_from_slice(&v.to_le_bytes());
        }
        w.extend_from_slice(&16_000u32.to_le_bytes());
        w.extend_from_slice(&64_000u32.to_le_bytes());
        w.extend_from_slice(&4u16.to_le_bytes());
        w.extend_from_slice(&16u16.to_le_bytes());
        w.extend_from_slice(b"data");
        w.extend_from_slice(&8u32.to_le_bytes());
        for s in [16384i16, 0, 0, 16384] {
            w.extend_from_slice(&s.to_le_bytes());
        }
        let (out, hz) = parse_wav(&w).unwrap();
        assert_eq!(hz, 16_000);
        assert_eq!(out.len(), 2);
        assert!((out[0] - 0.25).abs() < 0.001);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_wav(b"nada").is_err());
        assert!(parse_wav(&[0u8; 64]).is_err());
    }
}
