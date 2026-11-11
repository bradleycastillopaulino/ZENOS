//! Detección de actividad de voz por energía, con calibración de ruido
//! y pre-roll para no cortar las primeras sílabas.

use std::collections::VecDeque;

pub const SAMPLE_RATE: u32 = 16_000;
pub const FRAME: usize = 480; // 30 ms a 16 kHz
const FRAME_MS: u32 = 30;

#[derive(Debug, Clone)]
pub struct VadConfig {
    pub calibration_ms: u32,
    pub min_threshold: f32,
    pub noise_multiplier: f32,
    pub silence_ms: u32,
    pub max_ms: u32,
    pub start_timeout_ms: u32,
    pub pre_roll_ms: u32,
    pub min_speech_ms: u32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            calibration_ms: 300,
            min_threshold: 0.012,
            noise_multiplier: 3.0,
            silence_ms: 1200,
            max_ms: 30_000,
            start_timeout_ms: 7_000,
            pre_roll_ms: 450,
            min_speech_ms: 240,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum VadState {
    /// Seguir grabando.
    Continue,
    /// Habla terminada: el audio está listo.
    Done,
    /// No se detectó voz (o fue demasiado corta).
    NoSpeech,
}

pub struct Vad {
    cfg: VadConfig,
    elapsed_ms: u32,
    noise: Vec<f32>,
    threshold: f32,
    pre_roll: VecDeque<Vec<f32>>,
    started: bool,
    speech_ms: u32,
    silence_ms: u32,
    pub audio: Vec<f32>,
}

pub fn rms(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }
    (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt()
}

impl Vad {
    pub fn new(cfg: VadConfig) -> Self {
        let threshold = cfg.min_threshold;
        Self {
            cfg,
            elapsed_ms: 0,
            noise: Vec::new(),
            threshold,
            pre_roll: VecDeque::new(),
            started: false,
            speech_ms: 0,
            silence_ms: 0,
            audio: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn started(&self) -> bool {
        self.started
    }

    /// Procesa un bloque de 30 ms (FRAME muestras).
    pub fn push(&mut self, frame: &[f32]) -> VadState {
        self.elapsed_ms += FRAME_MS;
        let level = rms(frame);

        if !self.started {
            self.pre_roll.push_back(frame.to_vec());
            let max_pre = (self.cfg.pre_roll_ms / FRAME_MS).max(1) as usize;
            while self.pre_roll.len() > max_pre {
                self.pre_roll.pop_front();
            }
            if self.elapsed_ms <= self.cfg.calibration_ms {
                self.noise.push(level);
                let mean = self.noise.iter().sum::<f32>() / self.noise.len() as f32;
                self.threshold = (mean * self.cfg.noise_multiplier).max(self.cfg.min_threshold);
                return VadState::Continue;
            }
            if level > self.threshold {
                self.started = true;
                self.speech_ms = FRAME_MS;
                for f in self.pre_roll.drain(..) {
                    self.audio.extend_from_slice(&f);
                }
                return VadState::Continue;
            }
            if self.elapsed_ms >= self.cfg.start_timeout_ms {
                return VadState::NoSpeech;
            }
            return VadState::Continue;
        }

        self.audio.extend_from_slice(frame);
        if level > self.threshold {
            self.speech_ms += FRAME_MS;
            self.silence_ms = 0;
        } else {
            self.silence_ms += FRAME_MS;
        }
        if self.silence_ms >= self.cfg.silence_ms || self.elapsed_ms >= self.cfg.max_ms {
            return if self.speech_ms >= self.cfg.min_speech_ms { VadState::Done } else { VadState::NoSpeech };
        }
        VadState::Continue
    }

    /// Termina por orden externa (p. ej. el usuario pulsa el atajo para enviar).
    pub fn finish(&self) -> VadState {
        if self.started && self.speech_ms >= self.cfg.min_speech_ms {
            VadState::Done
        } else {
            VadState::NoSpeech
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(amp: f32) -> Vec<f32> {
        (0..FRAME).map(|i| amp * ((i as f32) * 0.2).sin()).collect()
    }

    #[test]
    fn detects_speech_then_silence() {
        let mut v = Vad::new(VadConfig::default());
        let quiet = tone(0.001);
        let loud = tone(0.3);
        for _ in 0..20 {
            assert_eq!(v.push(&quiet), VadState::Continue);
        }
        for _ in 0..30 {
            assert_eq!(v.push(&loud), VadState::Continue);
        }
        assert!(v.started());
        let mut result = VadState::Continue;
        for _ in 0..100 {
            result = v.push(&quiet);
            if result != VadState::Continue {
                break;
            }
        }
        assert_eq!(result, VadState::Done);
        // pre-roll incluido: más audio que solo la parte de voz
        assert!(v.audio.len() > 30 * FRAME);
    }

    #[test]
    fn times_out_without_speech() {
        let mut v = Vad::new(VadConfig { start_timeout_ms: 600, ..Default::default() });
        let quiet = tone(0.001);
        let mut r = VadState::Continue;
        for _ in 0..100 {
            r = v.push(&quiet);
            if r != VadState::Continue {
                break;
            }
        }
        assert_eq!(r, VadState::NoSpeech);
    }

    #[test]
    fn noisy_room_raises_threshold() {
        let mut v = Vad::new(VadConfig::default());
        let noise = tone(0.05);
        for _ in 0..10 {
            v.push(&noise);
        }
        // El mismo nivel de ruido no debe contar como voz.
        for _ in 0..10 {
            v.push(&noise);
        }
        assert!(!v.started());
    }

    #[test]
    fn short_click_is_discarded() {
        let mut v = Vad::new(VadConfig::default());
        let quiet = tone(0.001);
        for _ in 0..12 {
            v.push(&quiet);
        }
        v.push(&tone(0.5));
        let mut r = VadState::Continue;
        for _ in 0..100 {
            r = v.push(&quiet);
            if r != VadState::Continue {
                break;
            }
        }
        assert_eq!(r, VadState::NoSpeech);
    }
}
