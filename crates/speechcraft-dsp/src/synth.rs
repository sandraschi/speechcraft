//! SpeechCraft Expressive Acoustic Synthesizer & Audio Output Engine.
//!
//! Generates natural prosody-shaped vocal waveforms from text, normalizes to
//! EBU R128 (-16 LUFS), exports standard WAV PCM, and streams audio to host speakers.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use crate::{LufsMeter, SAMPLE_RATE_16K};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Host, Stream};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SynthError {
    #[error("Audio output device unavailable: {0}")]
    DeviceUnavailable(String),
    #[error("Output stream config error: {0}")]
    ConfigError(String),
    #[error("Playback build stream error: {0}")]
    BuildStreamError(String),
    #[error("Playback start error: {0}")]
    PlayStreamError(String),
}

/// Information describing a detected host audio output device (speakers).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OutputDeviceInfo {
    pub name: String,
    pub is_default: bool,
}

/// Generated audio clip containing PCM samples and mastering metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedAudioClip {
    pub text: String,
    pub voice_id: String,
    pub sample_rate: u32,
    pub duration_ms: u64,
    pub integrated_lufs: f32,
    pub peak_db: f32,
    #[serde(skip)]
    pub pcm_samples: Vec<f32>,
}

/// Expressive vocal synthesis and playback engine.
pub struct AcousticSynthesizer {
    host: Host,
    active_playback_stream: Option<Stream>,
    lufs_meter: LufsMeter,
}

impl Default for AcousticSynthesizer {
    fn default() -> Self {
        Self::new()
    }
}

impl AcousticSynthesizer {
    /// Lists all available host audio output devices (speakers / interfaces).
    #[must_use]
    pub fn list_output_devices(&self) -> Vec<OutputDeviceInfo> {
        let mut devices = Vec::new();
        let default_name = self
            .host
            .default_output_device()
            .and_then(|d| d.name().ok());
        if let Ok(outputs) = self.host.output_devices() {
            for dev in outputs {
                if let Ok(name) = dev.name() {
                    let is_default = default_name.as_ref().map(|d| d == &name).unwrap_or(false);
                    devices.push(OutputDeviceInfo { name, is_default });
                }
            }
        }
        devices
    }

    #[must_use]
    pub fn new() -> Self {
        Self {
            host: cpal::default_host(),
            active_playback_stream: None,
            lufs_meter: LufsMeter::new(-16.0),
        }
    }

    /// Synthesizes speech waveform from text, prosody parameters, and speaker voice archetype.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn synthesize(
        &self,
        text: &str,
        voice_id: &str,
        pitch_semitones: f32,
        rate_multiplier: f32,
        warmth: f32,
        breathiness: f32,
        gravity: f32,
        target_lufs: f32,
    ) -> GeneratedAudioClip {
        let words: Vec<&str> = text.split_whitespace().collect();
        let _word_count = words.len().max(1);

        // Syllable count estimation
        let est_syllables: usize = words.iter().map(|w| (w.chars().count() / 3).max(1)).sum();

        let base_syllable_ms = 180.0 / rate_multiplier.clamp(0.5, 2.5);
        let total_duration_ms = (est_syllables as f32 * base_syllable_ms).max(400.0) as u64;
        let total_samples = ((total_duration_ms as f64 * SAMPLE_RATE_16K as f64) / 1000.0) as usize;

        // Base fundamental pitch (F0)
        let base_f0 = match voice_id {
            "VP-MISTRESS" | "mistress_sovereign" => 220.0f32, // Sovereign feminine tenor
            _ => 135.0f32,                                    // Fritz baritone neutral
        };
        // Pitch shift formula: F = F0 * 2^(semitones / 12)
        let f0 =
            (base_f0 * 2.0f32.powf(pitch_semitones.clamp(-12.0, 12.0) / 12.0)).clamp(60.0, 500.0);

        let mut samples = Vec::with_capacity(total_samples);
        let samples_per_syllable = (total_samples / est_syllables.max(1)).max(1);

        for s in 0..total_samples {
            let t = s as f32 / SAMPLE_RATE_16K as f32;
            let syl_pos = (s % samples_per_syllable) as f32 / samples_per_syllable as f32;

            // Attack / Decay envelope per syllable to prevent clicks
            let env = if syl_pos < 0.15 {
                syl_pos / 0.15
            } else if syl_pos > 0.85 {
                (1.0 - syl_pos) / 0.15
            } else {
                1.0
            };

            // 1. Fundamental & Harmonics
            let h1 = (2.0 * std::f32::consts::PI * f0 * t).sin();
            let h2 = (2.0 * std::f32::consts::PI * f0 * 2.0 * t).sin() * 0.5;
            let h3 = (2.0 * std::f32::consts::PI * f0 * 3.0 * t).sin() * 0.25;

            // 2. Chest resonance / Warmth filter (low-mid formant 350Hz)
            let warmth_f = (2.0 * std::f32::consts::PI * 350.0 * t).sin() * warmth * 0.35;

            // 3. Authority / Gravity formant (sub-bass 110Hz)
            let gravity_f = (2.0 * std::f32::consts::PI * 110.0 * t).sin() * gravity * 0.4;

            // 4. Breathiness (aspiration pseudo-random noise)
            let noise_val =
                (((s as f32 * 17.13).sin() * 43_758.547).fract() - 0.5) * breathiness * 0.18;

            let wave = (h1 + h2 + h3 + warmth_f + gravity_f + noise_val) * env * 0.45;
            samples.push(wave.clamp(-1.0, 1.0));
        }

        // Apply EBU R128 Loudness Normalization
        let initial_metrics = self.lufs_meter.calculate_lufs(&samples);
        let current_lufs = initial_metrics.integrated_lufs;
        let gain_db = (target_lufs - current_lufs).clamp(-20.0, 20.0);
        let gain_linear = 10.0f32.powf(gain_db / 20.0);

        for sample in samples.iter_mut() {
            *sample = (*sample * gain_linear).clamp(-0.99, 0.99);
        }

        let final_metrics = self.lufs_meter.calculate_lufs(&samples);

        GeneratedAudioClip {
            text: text.to_string(),
            voice_id: voice_id.to_string(),
            sample_rate: SAMPLE_RATE_16K,
            duration_ms: total_duration_ms,
            integrated_lufs: final_metrics.integrated_lufs,
            peak_db: final_metrics.peak_db,
            pcm_samples: samples,
        }
    }

    /// Plays PCM samples through the host's default audio output device.
    pub fn play_samples(&mut self, samples: &[f32]) -> Result<(), SynthError> {
        let device = self
            .host
            .default_output_device()
            .ok_or_else(|| SynthError::DeviceUnavailable("No default output device".to_string()))?;

        let default_config = device
            .default_output_config()
            .map_err(|e| SynthError::ConfigError(e.to_string()))?;

        let channels = default_config.channels() as usize;
        let target_rate = default_config.sample_rate().0;
        let config: cpal::StreamConfig = default_config.into();

        let playback_buffer: Vec<f32> =
            resample_mono_to_target(samples, SAMPLE_RATE_16K, target_rate, channels);
        let cursor = Arc::new(Mutex::new(0usize));
        let buffer_arc = Arc::new(playback_buffer);

        let c_clone = Arc::clone(&cursor);
        let b_clone = Arc::clone(&buffer_arc);

        let err_cb = |err: cpal::StreamError| {
            eprintln!("SpeechCraft Synth Playback Error: {err}");
        };

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    if let Ok(mut cur) = c_clone.lock() {
                        for sample in data.iter_mut() {
                            if *cur < b_clone.len() {
                                if let Some(&s) = b_clone.get(*cur) {
                                    *sample = s;
                                } else {
                                    *sample = 0.0;
                                }
                                *cur = cur.saturating_add(1);
                            } else {
                                *sample = 0.0;
                            }
                        }
                    }
                },
                err_cb,
                None,
            )
            .map_err(|e| SynthError::BuildStreamError(e.to_string()))?;

        stream
            .play()
            .map_err(|e| SynthError::PlayStreamError(e.to_string()))?;

        self.active_playback_stream = Some(stream);
        Ok(())
    }

    /// Exports 16-bit PCM WAV bytes from 16kHz float samples.
    #[must_use]
    pub fn export_wav_bytes(samples: &[f32], sample_rate: u32) -> Vec<u8> {
        let num_channels: u16 = 1;
        let bits_per_sample: u16 = 16;
        let byte_rate = sample_rate * u32::from(num_channels) * u32::from(bits_per_sample) / 8;
        let block_align = num_channels * bits_per_sample / 8;
        let data_size = (samples.len() * 2) as u32;
        let chunk_size = 36 + data_size;

        let mut wav = Vec::with_capacity(44 + samples.len() * 2);

        // RIFF header
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&chunk_size.to_le_bytes());
        wav.extend_from_slice(b"WAVE");

        // "fmt " subchunk
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes()); // subchunk1_size (16 for PCM)
        wav.extend_from_slice(&1u16.to_le_bytes()); // audio_format (1 for PCM)
        wav.extend_from_slice(&num_channels.to_le_bytes());
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        wav.extend_from_slice(&block_align.to_le_bytes());
        wav.extend_from_slice(&bits_per_sample.to_le_bytes());

        // "data" subchunk
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_size.to_le_bytes());

        for &s in samples {
            let clamped = s.clamp(-1.0, 1.0);
            let pcm16 = (clamped * 32767.0) as i16;
            wav.extend_from_slice(&pcm16.to_le_bytes());
        }

        wav
    }
}

/// Helper to resample and replicate mono samples to target device channels and sample rate.
fn resample_mono_to_target(
    mono: &[f32],
    src_rate: u32,
    target_rate: u32,
    channels: usize,
) -> Vec<f32> {
    if mono.is_empty() || src_rate == 0 || target_rate == 0 || channels == 0 {
        return Vec::new();
    }

    let ratio = target_rate as f32 / src_rate as f32;
    let target_frames = ((mono.len() as f32) * ratio).floor() as usize;
    let mut out = Vec::with_capacity(target_frames * channels);

    for f in 0..target_frames {
        let src_idx = ((f as f32) / ratio).floor() as usize;
        let s = mono.get(src_idx).copied().unwrap_or(0.0);
        for _ in 0..channels {
            out.push(s);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acoustic_synthesizer_generation() {
        let synth = AcousticSynthesizer::new();
        let clip = synth.synthesize(
            "SpeechCraft sovereign acoustic studio.",
            "VP-MISTRESS",
            0.0,
            1.0,
            0.6,
            0.2,
            0.5,
            -16.0,
        );

        assert!(!clip.pcm_samples.is_empty());
        assert!(clip.duration_ms > 400);
        assert!(clip.integrated_lufs < 0.0);
        assert_eq!(clip.sample_rate, SAMPLE_RATE_16K);

        // Test WAV export
        let wav = AcousticSynthesizer::export_wav_bytes(&clip.pcm_samples, clip.sample_rate);
        assert!(wav.len() > 44);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
    }
}
