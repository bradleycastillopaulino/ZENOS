//! Utilidades de audio: mezcla a mono, remuestreo y codificación WAV.

/// Convierte audio entrelazado de `channels` canales a mono.
pub fn to_mono(data: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return data.to_vec();
    }
    data.chunks(channels).map(|c| c.iter().sum::<f32>() / c.len() as f32).collect()
}

/// Remuestreador lineal en streaming (suficiente para voz).
pub struct Resampler {
    step: f64,
    pos: f64,
    last: f32,
}

impl Resampler {
    pub fn new(from_hz: u32, to_hz: u32) -> Self {
        Self { step: from_hz as f64 / to_hz as f64, pos: 0.0, last: 0.0 }
    }

    pub fn process(&mut self, input: &[f32]) -> Vec<f32> {
        if (self.step - 1.0).abs() < f64::EPSILON {
            return input.to_vec();
        }
        let mut out = Vec::with_capacity((input.len() as f64 / self.step) as usize + 2);
        // `pos` es relativo a la muestra previa (`last`, índice -1).
        while self.pos < input.len() as f64 {
            let i = self.pos.floor();
            let frac = (self.pos - i) as f32;
            let i = i as isize;
            let a = if i == 0 { self.last } else { input[(i - 1) as usize] };
            let b = input[i as usize];
            out.push(a + (b - a) * frac);
            self.pos += self.step;
        }
        self.pos -= input.len() as f64;
        if let Some(&l) = input.last() {
            self.last = l;
        }
        out
    }
}

/// WAV PCM 16 bits mono.
pub fn encode_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_mix() {
        assert_eq!(to_mono(&[1.0, 0.0, 0.5, 0.5], 2), vec![0.5, 0.5]);
    }

    #[test]
    fn resample_48k_to_16k_keeps_duration() {
        let mut r = Resampler::new(48_000, 16_000);
        let mut total = 0;
        for _ in 0..100 {
            total += r.process(&vec![0.1; 480]).len();
        }
        // 48000 muestras -> ~16000
        assert!((total as i64 - 16_000).abs() <= 2, "{total}");
    }

    #[test]
    fn resample_44100_preserves_signal_level() {
        let mut r = Resampler::new(44_100, 16_000);
        let out = r.process(&vec![0.25; 4410]);
        assert!(out.iter().skip(1).all(|s| (s - 0.25).abs() < 1e-6));
    }

    #[test]
    fn wav_header() {
        let w = encode_wav(&[0.0, 1.0, -1.0], 16_000);
        assert_eq!(&w[0..4], b"RIFF");
        assert_eq!(&w[8..12], b"WAVE");
        assert_eq!(w.len(), 44 + 6);
        assert_eq!(u32::from_le_bytes(w[24..28].try_into().unwrap()), 16_000);
        assert_eq!(i16::from_le_bytes([w[46], w[47]]), i16::MAX);
    }
}
