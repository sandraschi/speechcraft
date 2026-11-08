//! SpeechCraft Wake Word Engine & Coordinator.
//!
//! Subsumes openWakeWord and Picovoice capabilities in pure Rust.
//! Handles acoustic frame verification, VAD thresholds, spoken greeting acknowledgments,
//! and sleep keywords without external cloud reliance.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use crate::models::{SpeechIntent, WakeWordConfig};
use speechcraft_dsp::{AudioRingBuffer, KeywordMatcher, VadEngine, KEYWORD_WINDOW_SAMPLES};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeState {
    Dormant,
    ListeningForWake,
    AwakeCapturingCommand { started_epoch_ms: u64 },
    Sleeping,
}

#[derive(Debug, Clone)]
pub struct WakeWordCoordinator {
    pub config: WakeWordConfig,
    pub state: WakeState,
    pub ring_buffer: AudioRingBuffer,
    pub vad: VadEngine,
    pub keyword_matcher: KeywordMatcher,
    pub last_detected_keyword: Option<String>,
    pub last_keyword_confidence: f32,
    pub last_vad_rms: f32,
    pub wake_event_count: usize,
}

impl Default for WakeWordCoordinator {
    fn default() -> Self {
        Self::new(WakeWordConfig::default())
    }
}

impl WakeWordCoordinator {
    #[must_use]
    pub fn new(config: WakeWordConfig) -> Self {
        let vad = VadEngine::new(0.015, config.vad_threshold);
        let state = if config.is_listening {
            WakeState::ListeningForWake
        } else {
            WakeState::Sleeping
        };
        Self {
            config,
            state,
            ring_buffer: AudioRingBuffer::new(16_000 * 10), // 10 seconds capacity
            vad,
            keyword_matcher: KeywordMatcher::new(),
            last_detected_keyword: None,
            last_keyword_confidence: 0.0,
            last_vad_rms: 0.0,
            wake_event_count: 0,
        }
    }

    /// Ingests incoming 16kHz audio frame and evaluates wake/sleep transitions.
    pub fn ingest_samples(&mut self, samples: &[f32], current_epoch_ms: u64) -> Option<String> {
        self.ring_buffer.push_samples(samples);
        let metrics = self.vad.analyze_frame(samples);
        self.last_vad_rms = metrics.energy_rms;

        match self.state {
            WakeState::ListeningForWake => {
                if metrics.is_speech {
                    let recent = self.ring_buffer.read_recent(KEYWORD_WINDOW_SAMPLES);
                    let keyword_name = self.config.keyword.as_str().to_string();
                    let match_res = self.keyword_matcher.evaluate_keyword(
                        &recent,
                        &keyword_name,
                        self.config.sensitivity,
                    );
                    self.last_keyword_confidence = match_res.confidence;

                    // Trigger wake if acoustic template matches
                    if match_res.is_detected {
                        self.last_detected_keyword = Some(keyword_name);
                        self.wake_event_count = self.wake_event_count.saturating_add(1);
                        self.state = WakeState::AwakeCapturingCommand {
                            started_epoch_ms: current_epoch_ms,
                        };
                        return Some(format!("WAKE:{}", self.config.wake_greeting));
                    }
                }
            }
            WakeState::AwakeCapturingCommand { started_epoch_ms } => {
                let duration_sec = current_epoch_ms.saturating_sub(started_epoch_ms) / 1000;
                if duration_sec >= self.config.listen_command_seconds as u64 {
                    self.state = WakeState::ListeningForWake;
                    return Some("COMMAND_WINDOW_EXPIRED".to_string());
                }
            }
            WakeState::Dormant | WakeState::Sleeping => {}
        }

        None
    }

    /// Triggers a manual or spoken sleep event.
    pub fn trigger_sleep(&mut self) -> String {
        self.state = WakeState::Sleeping;
        "Going to sleep.".to_string()
    }

    /// Triggers wake state to active listening.
    pub fn trigger_wake(&mut self) -> String {
        self.state = WakeState::ListeningForWake;
        self.config.wake_greeting.clone()
    }

    /// Constructs a speech intent from the command utterance buffer.
    #[must_use]
    pub fn extract_intent(&self, transcript: &str, current_epoch: u64) -> SpeechIntent {
        let trimmed = transcript.trim();
        let (entity, command) = if let Some((ent, cmd)) = trimmed.split_once(' ') {
            let ent_lower = ent.to_lowercase();
            match ent_lower.as_str() {
                "fritz" | "codercraft" | "robotcraft" | "opencode" | "calibre" | "plexy"
                | "dreame" | "boomy" => (ent_lower, cmd.to_string()),
                _ => ("fritz".to_string(), trimmed.to_string()),
            }
        } else {
            ("fritz".to_string(), trimmed.to_string())
        };

        SpeechIntent {
            intent_id: format!("INTENT-{current_epoch}"),
            receiver_entity: entity,
            raw_transcript: trimmed.to_string(),
            action_command: command,
            confidence: 0.96,
            timestamp_epoch: current_epoch,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wake_coordinator_lifecycle() {
        let mut coord = WakeWordCoordinator::default();
        assert_eq!(coord.state, WakeState::ListeningForWake);

        // Feed speech frame to wake
        let speech_frame = vec![0.2; 1280];
        let event = coord.ingest_samples(&speech_frame, 1000);
        assert!(event.is_some());
        assert_eq!(coord.wake_event_count, 1);

        let intent = coord.extract_intent("robotcraft forward 50", 1005);
        assert_eq!(intent.receiver_entity, "robotcraft");
        assert_eq!(intent.action_command, "forward 50");

        let bare_intent = coord.extract_intent("gpu status", 1006);
        assert_eq!(bare_intent.receiver_entity, "fritz");
        assert_eq!(bare_intent.action_command, "gpu status");

        let sleep_msg = coord.trigger_sleep();
        assert_eq!(sleep_msg, "Going to sleep.");
        assert_eq!(coord.state, WakeState::Sleeping);
    }
}
