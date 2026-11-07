//! SpeechCraft Acoustic Voiceprint & Biometric Enclave.
//!
//! Generates 32-dimensional acoustic biometric embeddings from 16kHz audio
//! and computes sovereign verification scores across enrolled voiceprints.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use serde::{Deserialize, Serialize};

pub const VOICEPRINT_EMBEDDING_DIM: usize = 32;
pub const BIOMETRIC_MATCH_THRESHOLD: f32 = 0.72;

/// Voiceprint match verification result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceprintMatchResult {
    pub profile_id: String,
    pub similarity: f32,
    pub is_authenticated: bool,
    pub threshold: f32,
}

/// Enclave acoustic biometric engine.
#[derive(Debug, Clone)]
pub struct VoiceprintEnclave {
    pub threshold: f32,
}

impl Default for VoiceprintEnclave {
    fn default() -> Self {
        Self::new(BIOMETRIC_MATCH_THRESHOLD)
    }
}

impl VoiceprintEnclave {
    #[must_use]
    pub fn new(threshold: f32) -> Self {
        Self {
            threshold: threshold.clamp(0.1, 0.99),
        }
    }

    /// Extracts a 32-dimensional acoustic embedding from 16kHz audio samples.
    #[must_use]
    pub fn extract_embedding(&self, samples: &[f32]) -> Vec<f32> {
        let mut embedding = vec![0.0f32; VOICEPRINT_EMBEDDING_DIM];
        if samples.is_empty() {
            return embedding;
        }

        let chunk_size = (samples.len() / VOICEPRINT_EMBEDDING_DIM).max(1);
        for (i, slot) in embedding.iter_mut().enumerate() {
            let start = i * chunk_size;
            let end = (start + chunk_size).min(samples.len());
            let sub = samples.get(start..end).unwrap_or(&[]);

            let mut energy = 0.0f32;
            let mut zcr_count = 0usize;

            for j in 0..sub.len() {
                if let Some(&s) = sub.get(j) {
                    energy += s * s;
                    if j > 0 {
                        if let Some(&prev) = sub.get(j - 1) {
                            if (prev >= 0.0 && s < 0.0) || (prev < 0.0 && s >= 0.0) {
                                zcr_count = zcr_count.saturating_add(1);
                            }
                        }
                    }
                }
            }

            let rms = if !sub.is_empty() {
                (energy / sub.len() as f32).sqrt()
            } else {
                0.0
            };
            let zcr = if !sub.is_empty() {
                zcr_count as f32 / sub.len() as f32
            } else {
                0.0
            };

            // Non-linear acoustic mapping
            *slot = (rms * 0.7 + zcr * 0.3).clamp(0.0, 1.0);
        }

        // Normalize to unit sphere
        Self::normalize(&mut embedding);
        embedding
    }

    /// Generates reference embedding for enrolled voiceprint archetypes.
    #[must_use]
    pub fn generate_archetype_embedding(base_seed: f32, timbre_weight: f32) -> Vec<f32> {
        let mut emb = Vec::with_capacity(VOICEPRINT_EMBEDDING_DIM);
        for i in 0..VOICEPRINT_EMBEDDING_DIM {
            let angle = (i as f32 * 0.2 + base_seed) * std::f32::consts::PI;
            let val = (angle.sin() * 0.6 + timbre_weight * 0.4 + 0.5).clamp(0.05, 0.95);
            emb.push(val);
        }
        Self::normalize(&mut emb);
        emb
    }

    /// Normalizes vector in-place.
    fn normalize(v: &mut [f32]) {
        let sum_sq: f32 = v.iter().map(|&x| x * x).sum();
        let norm = sum_sq.sqrt();
        if norm > 1e-6 {
            for x in v.iter_mut() {
                *x /= norm;
            }
        }
    }

    /// Computes cosine similarity between two acoustic embeddings.
    #[must_use]
    pub fn compute_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let dot: f32 = a.iter().zip(b.iter()).map(|(&x, &y)| x * y).sum();
        dot.clamp(0.0, 1.0)
    }

    /// Verifies input samples against an enrolled biometric profile embedding.
    #[must_use]
    pub fn verify(
        &self,
        samples: &[f32],
        enrolled_embedding: &[f32],
        profile_id: &str,
    ) -> VoiceprintMatchResult {
        let input_embedding = self.extract_embedding(samples);
        let similarity = Self::compute_similarity(&input_embedding, enrolled_embedding);
        let is_authenticated = similarity >= self.threshold;

        VoiceprintMatchResult {
            profile_id: profile_id.to_string(),
            similarity,
            is_authenticated,
            threshold: self.threshold,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voiceprint_embedding_extraction() {
        let enclave = VoiceprintEnclave::default();
        let samples = vec![0.1f32; 16_000];
        let emb = enclave.extract_embedding(&samples);
        assert_eq!(emb.len(), VOICEPRINT_EMBEDDING_DIM);

        // Verify unit norm
        let norm_sq: f32 = emb.iter().map(|&x| x * x).sum();
        assert!((norm_sq.sqrt() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_voiceprint_archetype_and_verification() {
        let _enclave = VoiceprintEnclave::new(0.70);
        let mistress_emb = VoiceprintEnclave::generate_archetype_embedding(0.25, 0.8);
        assert_eq!(mistress_emb.len(), VOICEPRINT_EMBEDDING_DIM);

        // Self-similarity must be ~1.0
        let self_sim = VoiceprintEnclave::compute_similarity(&mistress_emb, &mistress_emb);
        assert!((self_sim - 1.0).abs() < 1e-4);

        // Different archetype
        let fritz_emb = VoiceprintEnclave::generate_archetype_embedding(0.75, 0.3);
        let cross_sim = VoiceprintEnclave::compute_similarity(&mistress_emb, &fritz_emb);
        assert!(cross_sim < 0.95);
    }
}
