//! SpeechCraft Hardware Audio Capture Engine.
//!
//! Enumerate input devices and stream live microphone samples into
//! the 16kHz mono ring buffer via pure-Rust CPAL WASAPI/DirectSound.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use crate::{AudioRingBuffer, SAMPLE_RATE_16K};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, Stream};
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioCaptureError {
    #[error("No audio host available: {0}")]
    HostUnavailable(String),
    #[error("Device not found: {0}")]
    DeviceNotFound(String),
    #[error("Default input device not found")]
    NoDefaultInputDevice,
    #[error("Failed to query input stream config: {0}")]
    ConfigError(String),
    #[error("Failed to build input stream: {0}")]
    BuildStreamError(String),
    #[error("Failed to play input stream: {0}")]
    PlayStreamError(String),
}

/// Information describing a detected hardware audio input device.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InputDeviceInfo {
    pub name: String,
    pub is_default: bool,
}

/// Sovereign hardware audio capture manager.
pub struct AudioCaptureManager {
    host: Host,
    active_stream: Option<Stream>,
    active_device_name: Option<String>,
}

impl AudioCaptureManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            host: cpal::default_host(),
            active_stream: None,
            active_device_name: None,
        }
    }

    /// Lists all available physical audio input devices.
    #[must_use]
    pub fn list_input_devices(&self) -> Vec<InputDeviceInfo> {
        let mut devices = Vec::new();
        let default_name = self
            .host
            .default_input_device()
            .and_then(|d| d.name().ok());

        if let Ok(input_devs) = self.host.input_devices() {
            for dev in input_devs {
                if let Ok(name) = dev.name() {
                    let is_default = default_name.as_ref().map(|d| d == &name).unwrap_or(false);
                    devices.push(InputDeviceInfo { name, is_default });
                }
            }
        }
        devices
    }

    /// Gets the name of the currently active recording device, if any.
    #[must_use]
    pub fn active_device_name(&self) -> Option<&str> {
        self.active_device_name.as_deref()
    }

    /// Returns whether the audio capture stream is currently active.
    #[must_use]
    pub fn is_capturing(&self) -> bool {
        self.active_stream.is_some()
    }

    /// Stops any currently running capture stream.
    pub fn stop_capture(&mut self) {
        self.active_stream = None;
        self.active_device_name = None;
    }

    /// Resolves target device: either named or default.
    fn resolve_device(&self, device_name: Option<&str>) -> Result<Device, AudioCaptureError> {
        if let Some(target) = device_name {
            let devs = self
                .host
                .input_devices()
                .map_err(|e| AudioCaptureError::HostUnavailable(e.to_string()))?;
            for dev in devs {
                if let Ok(name) = dev.name() {
                    if name.eq_ignore_ascii_case(target) {
                        return Ok(dev);
                    }
                }
            }
            return Err(AudioCaptureError::DeviceNotFound(target.to_string()));
        }

        self.host
            .default_input_device()
            .ok_or(AudioCaptureError::NoDefaultInputDevice)
    }

    /// Starts streaming from the selected microphone device into the thread-safe ring buffer.
    ///
    /// Automatically converts multi-channel audio to mono and resamples to 16kHz.
    pub fn start_capture(
        &mut self,
        device_name: Option<&str>,
        ring_buffer: Arc<Mutex<AudioRingBuffer>>,
    ) -> Result<String, AudioCaptureError> {
        self.stop_capture();

        let device = self.resolve_device(device_name)?;
        let resolved_name = device
            .name()
            .unwrap_or_else(|_| "Unknown Input Device".to_string());

        let default_config = device
            .default_input_config()
            .map_err(|e| AudioCaptureError::ConfigError(e.to_string()))?;

        let sample_format = default_config.sample_format();
        let channels = default_config.channels() as usize;
        let source_sample_rate = default_config.sample_rate().0;
        let config: cpal::StreamConfig = default_config.into();

        let err_callback = |err: cpal::StreamError| {
            eprintln!("SpeechCraft AudioCapture error: {err}");
        };

        let stream = match sample_format {
            cpal::SampleFormat::F32 => {
                let ring = Arc::clone(&ring_buffer);
                device.build_input_stream(
                    &config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        let mono_16k = resample_and_downmix(data, channels, source_sample_rate);
                        if let Ok(mut lock) = ring.lock() {
                            lock.push_samples(&mono_16k);
                        }
                    },
                    err_callback,
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                let ring = Arc::clone(&ring_buffer);
                device.build_input_stream(
                    &config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        let float_data: Vec<f32> =
                            data.iter().map(|&s| s as f32 / 32768.0).collect();
                        let mono_16k =
                            resample_and_downmix(&float_data, channels, source_sample_rate);
                        if let Ok(mut lock) = ring.lock() {
                            lock.push_samples(&mono_16k);
                        }
                    },
                    err_callback,
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                let ring = Arc::clone(&ring_buffer);
                device.build_input_stream(
                    &config,
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        let float_data: Vec<f32> =
                            data.iter().map(|&s| (s as f32 - 32768.0) / 32768.0).collect();
                        let mono_16k =
                            resample_and_downmix(&float_data, channels, source_sample_rate);
                        if let Ok(mut lock) = ring.lock() {
                            lock.push_samples(&mono_16k);
                        }
                    },
                    err_callback,
                    None,
                )
            }
            _ => {
                return Err(AudioCaptureError::ConfigError(format!(
                    "Unsupported sample format: {sample_format:?}"
                )))
            }
        }
        .map_err(|e| AudioCaptureError::BuildStreamError(e.to_string()))?;

        stream
            .play()
            .map_err(|e| AudioCaptureError::PlayStreamError(e.to_string()))?;

        self.active_stream = Some(stream);
        self.active_device_name = Some(resolved_name.clone());

        Ok(resolved_name)
    }
}

impl Default for AudioCaptureManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Converts interleaved multi-channel samples to mono and resamples to 16kHz.
#[must_use]
pub fn resample_and_downmix(samples: &[f32], channels: usize, source_rate: u32) -> Vec<f32> {
    if samples.is_empty() || channels == 0 || source_rate == 0 {
        return Vec::new();
    }

    // 1. Downmix interleaved channels to mono
    let frame_count = samples.len() / channels;
    let mut mono = Vec::with_capacity(frame_count);
    for f in 0..frame_count {
        let base = f * channels;
        let mut sum = 0.0f32;
        for c in 0..channels {
            if let Some(&s) = samples.get(base + c) {
                sum += s;
            }
        }
        mono.push(sum / channels as f32);
    }

    // 2. Resample mono to 16kHz
    if source_rate == SAMPLE_RATE_16K {
        return mono;
    }

    let ratio = source_rate as f32 / SAMPLE_RATE_16K as f32;
    let target_len = ((mono.len() as f32) / ratio).floor() as usize;
    let mut resampled = Vec::with_capacity(target_len);

    for i in 0..target_len {
        let src_idx = (i as f32 * ratio).floor() as usize;
        let sample = mono.get(src_idx).copied().unwrap_or(0.0);
        resampled.push(sample);
    }

    resampled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resample_and_downmix() {
        // Stereo 32kHz -> Mono 16kHz
        let stereo = vec![0.5, 0.5, -0.5, -0.5, 0.2, 0.2, -0.2, -0.2];
        let out = resample_and_downmix(&stereo, 2, 32_000);
        assert_eq!(out.len(), 2);
        assert!((out[0] - 0.5).abs() < 1e-4);
        assert!((out[1] - 0.2).abs() < 1e-4);
    }

    #[test]
    fn test_audio_capture_manager_init() {
        let manager = AudioCaptureManager::new();
        assert!(!manager.is_capturing());
        let _devices = manager.list_input_devices();
    }
}
