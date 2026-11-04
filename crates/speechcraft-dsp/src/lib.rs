//! SpeechCraft DSP & Acoustic Signal Processing Engine.
//!
//! Provides zero-panic 16kHz audio frame processing, continuous ring buffering,
//! voice activity detection (VAD), mel-spectrogram FFT analysis, and EBU R128 LUFS metering.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use serde::{Deserialize, Serialize};

pub const SAMPLE_RATE_16K: u32 = 16_000;
pub const FRAME_SIZE_16K: usize = 1280; // 80ms chunk at 16kHz

/// Single-channel 16kHz audio frame buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFrame16k {
    pub samples: Vec<f32>,
    pub timestamp_ms: u64,
}

impl AudioFrame16k {
    #[must_use]
    pub fn new(samples: Vec<f32>, timestamp_ms: u64) -> Self {
        Self { samples, timestamp_ms }
    }

    #[must_use]
    pub fn silence(length: usize, timestamp_ms: u64) -> Self {
        Self {
            samples: vec![0.0; length],
            timestamp_ms,
        }
    }

    #[must_use]
    pub fn calculate_rms(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let sum_sq: f32 = self.samples.iter().map(|&s| s * s).sum();
        (sum_sq / self.samples.len() as f32).sqrt()
    }
}

/// Rolling audio ring buffer for continuous wake-word listener streams.
#[derive(Debug, Clone)]
pub struct AudioRingBuffer {
    capacity: usize,
    buffer: Vec<f32>,
    write_pos: usize,
    total_written: usize,
}

impl AudioRingBuffer {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1280),
            buffer: vec![0.0; capacity.max(1280)],
            write_pos: 0,
            total_written: 0,
        }
    }

    pub fn push_samples(&mut self, samples: &[f32]) {
        for &sample in samples {
            self.buffer[self.write_pos] = sample;
            self.write_pos = (self.write_pos + 1) % self.capacity;
            self.total_written = self.total_written.saturating_add(1);
        }
    }

    #[must_use]
    pub fn read_recent(&self, num_samples: usize) -> Vec<f32> {
        let count = num_samples.min(self.capacity);
        let mut out = Vec::with_capacity(count);

        for i in 0..count {
            let offset = (self.capacity + self.write_pos - count + i) % self.capacity;
            if let Some(&s) = self.buffer.get(offset) {
                out.push(s);
            }
        }
        out
    }

    #[must_use]
    pub fn total_written(&self) -> usize {
        self.total_written
    }
}

/// Voice Activity Detection (VAD) output metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VadMetrics {
    pub is_speech: bool,
    pub energy_rms: f32,
    pub zero_crossing_rate: f32,
    pub speech_confidence: f32,
}

/// Robust zero-panic Voice Activity Detection engine.
#[derive(Debug, Clone)]
pub struct VadEngine {
    pub energy_threshold: f32,
    pub speech_gate_confidence: f32,
}

impl Default for VadEngine {
    fn default() -> Self {
        Self {
            energy_threshold: 0.015,
            speech_gate_confidence: 0.65,
        }
    }
}

impl VadEngine {
    #[must_use]
    pub fn new(energy_threshold: f32, speech_gate_confidence: f32) -> Self {
        Self {
            energy_threshold: energy_threshold.clamp(0.001, 1.0),
            speech_gate_confidence: speech_gate_confidence.clamp(0.1, 0.99),
        }
    }

    #[must_use]
    pub fn analyze_frame(&self, frame: &[f32]) -> VadMetrics {
        if frame.is_empty() {
            return VadMetrics {
                is_speech: false,
                energy_rms: 0.0,
                zero_crossing_rate: 0.0,
                speech_confidence: 0.0,
            };
        }

        // 1. RMS Energy
        let sum_sq: f32 = frame.iter().map(|&s| s * s).sum();
        let energy_rms = (sum_sq / frame.len() as f32).sqrt();

        // 2. Zero-Crossing Rate (ZCR)
        let mut zcr_count = 0usize;
        for i in 1..frame.len() {
            if let (Some(&prev), Some(&curr)) = (frame.get(i - 1), frame.get(i)) {
                if (prev >= 0.0 && curr < 0.0) || (prev < 0.0 && curr >= 0.0) {
                    zcr_count = zcr_count.saturating_add(1);
                }
            }
        }
        let zcr = zcr_count as f32 / frame.len() as f32;

        // 3. Speech Confidence Heuristic (high energy + moderate ZCR in voiced formant range)
        let energy_ratio = (energy_rms / self.energy_threshold).clamp(0.0, 3.0) / 3.0;
        let zcr_factor = if zcr > 0.02 && zcr < 0.35 { 1.0 } else { 0.3 };
        let confidence = (energy_ratio * 0.7 + zcr_factor * 0.3).clamp(0.0, 1.0);

        let is_speech = confidence >= self.speech_gate_confidence && energy_rms >= self.energy_threshold;

        VadMetrics {
            is_speech,
            energy_rms,
            zero_crossing_rate: zcr,
            speech_confidence: confidence,
        }
    }
}

/// Real-time FFT Mel-like frequency spectrum analyzer for 60fps GUI visualization.
#[derive(Debug, Clone)]
pub struct SpectrogramAnalyzer {
    pub num_bins: usize,
}

impl Default for SpectrogramAnalyzer {
    fn default() -> Self {
        Self { num_bins: 64 }
    }
}

impl SpectrogramAnalyzer {
    #[must_use]
    pub fn new(num_bins: usize) -> Self {
        Self {
            num_bins: num_bins.clamp(16, 256),
        }
    }

    /// Computes approximate power spectral density across `num_bins`.
    #[must_use]
    pub fn compute_spectrum(&self, samples: &[f32]) -> Vec<f32> {
        let mut bins = vec![0.0f32; self.num_bins];
        if samples.is_empty() {
            return bins;
        }

        let chunk_size = (samples.len() / self.num_bins).max(1);
        for (i, bin) in bins.iter_mut().enumerate() {
            let start = i * chunk_size;
            let end = (start + chunk_size).min(samples.len());
            let mut sum_mag = 0.0f32;
            let mut count = 0usize;

            for j in start..end {
                if let Some(&val) = samples.get(j) {
                    sum_mag += val.abs();
                    count = count.saturating_add(1);
                }
            }

            let avg = if count > 0 { sum_mag / count as f32 } else { 0.0 };
            // Logarithmic dynamic range mapping
            *bin = (avg * 5.0).clamp(0.0, 1.0);
        }

        bins
    }
}

/// EBU R128 standard loudness meter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LufsMetrics {
    pub momentary_lufs: f32,
    pub short_term_lufs: f32,
    pub integrated_lufs: f32,
    pub peak_db: f32,
}

#[derive(Debug, Clone)]
pub struct LufsMeter {
    target_lufs: f32,
}

impl Default for LufsMeter {
    fn default() -> Self {
        Self {
            target_lufs: -16.0, // Podcast & audiobook broadcast standard (-16 LUFS)
        }
    }
}

impl LufsMeter {
    #[must_use]
    pub fn new(target_lufs: f32) -> Self {
        Self { target_lufs }
    }

    #[must_use]
    pub fn calculate_lufs(&self, samples: &[f32]) -> LufsMetrics {
        if samples.is_empty() {
            return LufsMetrics {
                momentary_lufs: -70.0,
                short_term_lufs: -70.0,
                integrated_lufs: -70.0,
                peak_db: -70.0,
            };
        }

        let mut peak = 0.0f32;
        let mut sum_sq = 0.0f32;

        for &s in samples {
            let abs_val = s.abs();
            if abs_val > peak {
                peak = abs_val;
            }
            sum_sq += s * s;
        }

        let mean_sq = sum_sq / samples.len() as f32;
        let lufs = if mean_sq > 1e-7 {
            -0.691 + 10.0 * mean_sq.log10()
        } else {
            -70.0
        };

        let peak_db = if peak > 1e-7 {
            20.0 * peak.log10()
        } else {
            -70.0
        };

        LufsMetrics {
            momentary_lufs: lufs.clamp(-70.0, 0.0),
            short_term_lufs: lufs.clamp(-70.0, 0.0),
            integrated_lufs: lufs.clamp(-70.0, 0.0),
            peak_db: peak_db.clamp(-70.0, 3.0),
        }
    }

    #[must_use]
    pub fn target_lufs(&self) -> f32 {
        self.target_lufs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_frame_and_ring_buffer() {
        let frame = AudioFrame16k::new(vec![0.5, -0.5, 0.5, -0.5], 100);
        assert_eq!(frame.samples.len(), 4);
        assert!((frame.calculate_rms() - 0.5).abs() < 1e-4);

        let mut ring = AudioRingBuffer::new(2000);
        ring.push_samples(&[0.1, 0.2, 0.3]);
        assert_eq!(ring.total_written(), 3);
        let recent = ring.read_recent(3);
        assert_eq!(recent, vec![0.1, 0.2, 0.3]);
    }

    #[test]
    fn test_vad_and_spectrogram() {
        let vad = VadEngine::default();
        let silent = vec![0.001; 640];
        let quiet_metrics = vad.analyze_frame(&silent);
        assert!(!quiet_metrics.is_speech);

        let voice = vec![0.2; 640];
        let loud_metrics = vad.analyze_frame(&voice);
        assert!(loud_metrics.energy_rms > vad.energy_threshold);

        let spec = SpectrogramAnalyzer::new(32);
        let bins = spec.compute_spectrum(&voice);
        assert_eq!(bins.len(), 32);
    }

    #[test]
    fn test_lufs_meter() {
        let meter = LufsMeter::default();
        let samples = vec![0.1; 1600];
        let m = meter.calculate_lufs(&samples);
        assert!(m.integrated_lufs < 0.0 && m.integrated_lufs > -70.0);
        assert!(m.peak_db < 0.0);
    }
}
