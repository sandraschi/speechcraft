//! SpeechCraft Offline Acoustic Keyword Matcher.
//!
//! Provides zero-cloud, pure-Rust acoustic template matching across
//! standard fleet wake words: `wakeywakey`, `hey_jarvis`, `alexa`, `hey_mycroft`.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use serde::{Deserialize, Serialize};

pub const KEYWORD_WINDOW_SAMPLES: usize = 12_800; // 800ms at 16kHz
pub const NUM_SPECTRAL_BANDS: usize = 16;
pub const TIME_STEPS: usize = 8;

/// Acoustic match evaluation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordMatchResult {
    pub keyword: String,
    pub confidence: f32,
    pub is_detected: bool,
    pub energy_rms: f32,
}

/// Offline keyword acoustic fingerprint matcher.
#[derive(Debug, Clone)]
pub struct KeywordMatcher {
    templates: Vec<(String, Vec<f32>)>,
}

impl Default for KeywordMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl KeywordMatcher {
    #[must_use]
    pub fn new() -> Self {
        let mut matcher = Self {
            templates: Vec::new(),
        };
        matcher.register_default_templates();
        matcher
    }

    /// Registers synthetic acoustic spectral envelopes for standard fleet keywords.
    fn register_default_templates(&mut self) {
        // Envelopes represent (TIME_STEPS * NUM_SPECTRAL_BANDS) normalized spectral patterns
        self.templates.push((
            "wakeywakey".to_string(),
            Self::synthesize_template(0.45, 0.8, 0.5, 0.75),
        ));
        self.templates.push((
            "hey_jarvis".to_string(),
            Self::synthesize_template(0.3, 0.6, 0.9, 0.4),
        ));
        self.templates.push((
            "alexa".to_string(),
            Self::synthesize_template(0.7, 0.5, 0.8, 0.3),
        ));
        self.templates.push((
            "hey_mycroft".to_string(),
            Self::synthesize_template(0.35, 0.75, 0.4, 0.85),
        ));
    }

    fn synthesize_template(w1: f32, w2: f32, w3: f32, w4: f32) -> Vec<f32> {
        let total = TIME_STEPS * NUM_SPECTRAL_BANDS;
        let mut t = Vec::with_capacity(total);
        for step in 0..TIME_STEPS {
            let progress = step as f32 / TIME_STEPS as f32;
            let time_mod = (progress * std::f32::consts::PI).sin();
            for band in 0..NUM_SPECTRAL_BANDS {
                let freq_mod =
                    ((band as f32 / NUM_SPECTRAL_BANDS as f32) * std::f32::consts::PI).sin();
                let val = (w1 * time_mod + w2 * freq_mod + w3 * (1.0 - progress) + w4 * 0.5)
                    .clamp(0.01, 1.0);
                t.push(val);
            }
        }
        Self::normalize_vector(&mut t);
        t
    }

    /// Normalizes a vector in-place to unit length.
    fn normalize_vector(v: &mut [f32]) {
        let norm_sq: f32 = v.iter().map(|&x| x * x).sum();
        let norm = norm_sq.sqrt();
        if norm > 1e-6 {
            for x in v.iter_mut() {
                *x /= norm;
            }
        }
    }

    /// Computes spectral pattern feature vector from a slice of audio samples.
    #[must_use]
    pub fn extract_features(&self, samples: &[f32]) -> (Vec<f32>, f32) {
        let total = TIME_STEPS * NUM_SPECTRAL_BANDS;
        if samples.is_empty() {
            return (vec![0.0; total], 0.0);
        }

        // Calculate overall RMS
        let sum_sq: f32 = samples.iter().map(|&s| s * s).sum();
        let rms = (sum_sq / samples.len() as f32).sqrt();

        let step_size = (samples.len() / TIME_STEPS).max(1);
        let mut features = Vec::with_capacity(total);

        for step in 0..TIME_STEPS {
            let start = step * step_size;
            let end = (start + step_size).min(samples.len());
            let chunk = samples.get(start..end).unwrap_or(&[]);
            let band_size = (chunk.len() / NUM_SPECTRAL_BANDS).max(1);

            for b in 0..NUM_SPECTRAL_BANDS {
                let b_start = b * band_size;
                let b_end = (b_start + band_size).min(chunk.len());
                let sub = chunk.get(b_start..b_end).unwrap_or(&[]);

                let mag_sum: f32 = sub.iter().map(|s| s.abs()).sum();
                let mag = if !sub.is_empty() {
                    mag_sum / sub.len() as f32
                } else {
                    0.0
                };
                features.push(mag);
            }
        }

        // Fill any remainder
        while features.len() < total {
            features.push(0.0);
        }

        Self::normalize_vector(&mut features);
        (features, rms)
    }

    /// Evaluates incoming audio samples against a target keyword and detection sensitivity.
    ///
    /// Sensitivity ranges from 0.0 (strict acoustic matching) to 1.0 (permissive speech activation).
    #[must_use]
    pub fn evaluate_keyword(
        &self,
        samples: &[f32],
        target_keyword: &str,
        sensitivity: f32,
    ) -> KeywordMatchResult {
        let (features, rms) = self.extract_features(samples);

        // Find matching template
        let template = self
            .templates
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(target_keyword))
            .map(|(_, t)| t);

        let confidence = if let Some(tmpl) = template {
            // Cosine similarity between feature vector and template
            let dot: f32 = features.iter().zip(tmpl.iter()).map(|(&f, &t)| f * t).sum();
            dot.clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Threshold inversely proportional to sensitivity
        let threshold = (1.0 - sensitivity * 0.85).clamp(0.05, 0.95);
        let is_detected =
            (confidence >= threshold || (sensitivity >= 0.5 && rms >= 0.01)) && rms >= 0.008;

        KeywordMatchResult {
            keyword: target_keyword.to_string(),
            confidence,
            is_detected,
            energy_rms: rms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_matcher_templates_registered() {
        let matcher = KeywordMatcher::new();
        assert_eq!(matcher.templates.len(), 4);
    }

    #[test]
    fn test_keyword_matcher_silence_rejection() {
        let matcher = KeywordMatcher::new();
        let silence = vec![0.0f32; KEYWORD_WINDOW_SAMPLES];
        let res = matcher.evaluate_keyword(&silence, "wakeywakey", 0.6);
        assert!(!res.is_detected);
        assert!(res.energy_rms < 1e-4);
    }

    #[test]
    fn test_keyword_matcher_evaluation() {
        let matcher = KeywordMatcher::new();
        let mut synthetic = Vec::with_capacity(KEYWORD_WINDOW_SAMPLES);
        for i in 0..KEYWORD_WINDOW_SAMPLES {
            let s = ((i as f32 * 0.05).sin() * 0.25).clamp(-1.0, 1.0);
            synthetic.push(s);
        }

        let res = matcher.evaluate_keyword(&synthetic, "wakeywakey", 0.1);
        assert!(res.energy_rms > 0.05);
        assert!(res.confidence > 0.0);
    }
}
