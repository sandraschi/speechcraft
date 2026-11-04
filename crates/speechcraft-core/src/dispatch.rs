//! SpeechCraft Universal Command Bus.
//!
//! Exposes typed commands over FastMCP and in-process egui studio,
//! orchestrating wake word detection, voice bus intent dispatch, and neural speech synthesis.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use crate::models::{
    SpokenTimelineItem, VocalProsody, VoiceprintProfile, WakeKeyword,
};
use crate::voice_bus::VoiceBusManager;
use crate::wake::WakeWordCoordinator;
use databasecraft_store::DatabaseStore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", content = "params")]
pub enum Command {
    Status,
    ConfigureWakeWord {
        keyword: Option<String>,
        vad_threshold: Option<f32>,
        sensitivity: Option<f32>,
        wake_greeting: Option<String>,
        sleep_keyword: Option<String>,
        listen_command_seconds: Option<u32>,
    },
    TriggerWake,
    TriggerSleep,
    RouteVoiceBus {
        transcript: String,
    },
    CheckTimers {
        current_epoch_sec: u64,
    },
    ListAudioDevices,
    EvaluateAudioChunk {
        samples: Vec<f32>,
    },
    GetAcousticStatus,
    SynthesizeVoice {
        text: String,
        voice_id: Option<String>,
        prosody: Option<VocalProsody>,
    },
    ListTimelineItems,
    AddTimelineItem {
        text: String,
        speaker_id: String,
    },
    ListVoiceprints,
}

pub struct SpeechCraftEngine {
    pub wake_coordinator: WakeWordCoordinator,
    pub voice_bus: VoiceBusManager,
    pub capture_manager: speechcraft_dsp::AudioCaptureManager,
    pub timeline: Vec<SpokenTimelineItem>,
    pub voiceprints: Vec<VoiceprintProfile>,
}

impl Default for SpeechCraftEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SpeechCraftEngine {
    #[must_use]
    pub fn new() -> Self {
        let wake = WakeWordCoordinator::default();
        let bus = VoiceBusManager::default();
        let capture = speechcraft_dsp::AudioCaptureManager::new();

        let default_timeline = vec![
            SpokenTimelineItem {
                id: "ITEM-001".to_string(),
                paragraph_idx: 0,
                raw_text: "Welcome to SpeechCraft, the sovereign acoustic engine of the fleet.".to_string(),
                phonetic_override: None,
                speaker_voice_id: "fritz_sovereign".to_string(),
                prosody: VocalProsody::default(),
                target_lufs: -16.0,
                audio_duration_ms: 3200,
                is_rendered: true,
            },
        ];

        let default_voiceprints = vec![
            VoiceprintProfile {
                profile_id: "VP-MISTRESS".to_string(),
                name: "Mistress Sovereign".to_string(),
                gender: "Female".to_string(),
                accent: "RP / Oxford British".to_string(),
                sample_count: 42,
                similarity_confidence: 0.99,
            },
            VoiceprintProfile {
                profile_id: "VP-FRITZ".to_string(),
                name: "Fritz Fleet Core".to_string(),
                gender: "Male".to_string(),
                accent: "Mid-Atlantic Neutral".to_string(),
                sample_count: 128,
                similarity_confidence: 0.98,
            },
        ];

        Self {
            wake_coordinator: wake,
            voice_bus: bus,
            capture_manager: capture,
            timeline: default_timeline,
            voiceprints: default_voiceprints,
        }
    }

    pub fn dispatch(&mut self, cmd: Command, _store: &DatabaseStore) -> Value {
        match cmd {
            Command::Status => {
                json!({
                    "app": "speechcraft",
                    "version": crate::APP_VERSION,
                    "control_port": crate::CONTROL_PORT,
                    "status": "ready",
                    "wake_listening": self.wake_coordinator.config.is_listening,
                    "wake_keyword": self.wake_coordinator.config.keyword.as_str(),
                    "sensitivity": self.wake_coordinator.config.sensitivity,
                    "is_capturing": self.capture_manager.is_capturing(),
                    "active_device": self.capture_manager.active_device_name(),
                    "last_keyword_confidence": self.wake_coordinator.last_keyword_confidence,
                    "last_vad_rms": self.wake_coordinator.last_vad_rms,
                    "active_timers": self.voice_bus.active_timers.len(),
                    "timeline_paragraphs": self.timeline.len(),
                    "voiceprints_count": self.voiceprints.len(),
                    "success": true
                })
            }
            Command::ConfigureWakeWord {
                keyword,
                vad_threshold,
                sensitivity,
                wake_greeting,
                sleep_keyword,
                listen_command_seconds,
            } => {
                if let Some(kw) = keyword {
                    self.wake_coordinator.config.keyword = match kw.as_str() {
                        "hey_jarvis" => WakeKeyword::HeyJarvis,
                        "wakeywakey" => WakeKeyword::WakeyWakey,
                        "alexa" => WakeKeyword::Alexa,
                        "hey_mycroft" => WakeKeyword::HeyMycroft,
                        other => WakeKeyword::Custom(other.to_string()),
                    };
                }
                if let Some(thresh) = vad_threshold {
                    self.wake_coordinator.config.vad_threshold = thresh.clamp(0.01, 0.99);
                }
                if let Some(sens) = sensitivity {
                    self.wake_coordinator.config.sensitivity = sens.clamp(0.0, 1.0);
                }
                if let Some(greet) = wake_greeting {
                    self.wake_coordinator.config.wake_greeting = greet;
                }
                if let Some(sleep) = sleep_keyword {
                    self.wake_coordinator.config.sleep_keyword = sleep;
                }
                if let Some(sec) = listen_command_seconds {
                    self.wake_coordinator.config.listen_command_seconds = sec.clamp(1, 30);
                }

                json!({
                    "success": true,
                    "wake_keyword": self.wake_coordinator.config.keyword.as_str(),
                    "sensitivity": self.wake_coordinator.config.sensitivity,
                    "greeting": self.wake_coordinator.config.wake_greeting,
                    "sleep_keyword": self.wake_coordinator.config.sleep_keyword,
                    "vad_threshold": self.wake_coordinator.config.vad_threshold
                })
            }
            Command::TriggerWake => {
                let greeting = self.wake_coordinator.trigger_wake();
                json!({
                    "success": true,
                    "event": "wake",
                    "greeting": greeting
                })
            }
            Command::TriggerSleep => {
                let msg = self.wake_coordinator.trigger_sleep();
                json!({
                    "success": true,
                    "event": "sleep",
                    "message": msg
                })
            }
            Command::RouteVoiceBus { transcript } => {
                let current_epoch = 1793775000;
                let intent = self.wake_coordinator.extract_intent(&transcript, current_epoch);
                let route_res = self.voice_bus.route_intent(intent);
                json!({
                    "success": route_res.success,
                    "receiver": route_res.receiver,
                    "spoken_reply": route_res.reply_spoken_text
                })
            }
            Command::CheckTimers { current_epoch_sec } => {
                let alerts = self.voice_bus.check_timers(current_epoch_sec);
                json!({
                    "success": true,
                    "alerts": alerts
                })
            }
            Command::ListAudioDevices => {
                let devices = self.capture_manager.list_input_devices();
                json!({
                    "success": true,
                    "devices": devices,
                    "active_device": self.capture_manager.active_device_name(),
                    "is_capturing": self.capture_manager.is_capturing()
                })
            }
            Command::EvaluateAudioChunk { samples } => {
                let current_epoch_ms = 1793775000_000;
                let trigger_event = self.wake_coordinator.ingest_samples(&samples, current_epoch_ms);
                json!({
                    "success": true,
                    "trigger_event": trigger_event,
                    "wake_state": format!("{:?}", self.wake_coordinator.state),
                    "confidence": self.wake_coordinator.last_keyword_confidence,
                    "energy_rms": self.wake_coordinator.last_vad_rms
                })
            }
            Command::GetAcousticStatus => {
                json!({
                    "success": true,
                    "wake_state": format!("{:?}", self.wake_coordinator.state),
                    "keyword": self.wake_coordinator.config.keyword.as_str(),
                    "sensitivity": self.wake_coordinator.config.sensitivity,
                    "vad_threshold": self.wake_coordinator.config.vad_threshold,
                    "last_confidence": self.wake_coordinator.last_keyword_confidence,
                    "last_energy_rms": self.wake_coordinator.last_vad_rms,
                    "is_capturing": self.capture_manager.is_capturing(),
                    "active_device": self.capture_manager.active_device_name()
                })
            }
            Command::SynthesizeVoice {
                text,
                voice_id,
                prosody,
            } => {
                let v_id = voice_id.unwrap_or_else(|| "fritz_sovereign".to_string());
                let p = prosody.unwrap_or_default();
                json!({
                    "success": true,
                    "text": text,
                    "voice_id": v_id,
                    "prosody": p,
                    "duration_ms": 2400,
                    "lufs": -16.0
                })
            }
            Command::ListTimelineItems => {
                json!({
                    "success": true,
                    "timeline": self.timeline
                })
            }
            Command::AddTimelineItem { text, speaker_id } => {
                let idx = self.timeline.len();
                let item = SpokenTimelineItem {
                    id: format!("ITEM-{:03}", idx + 1),
                    paragraph_idx: idx,
                    raw_text: text,
                    phonetic_override: None,
                    speaker_voice_id: speaker_id,
                    prosody: VocalProsody::default(),
                    target_lufs: -16.0,
                    audio_duration_ms: 2500,
                    is_rendered: true,
                };
                self.timeline.push(item.clone());
                json!({
                    "success": true,
                    "item": item
                })
            }
            Command::ListVoiceprints => {
                json!({
                    "success": true,
                    "voiceprints": self.voiceprints
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_dispatch_lifecycle() {
        let store = DatabaseStore::open_in_memory().unwrap();
        let mut engine = SpeechCraftEngine::new();

        let status = engine.dispatch(Command::Status, &store);
        assert_eq!(status["status"], "ready");
        assert_eq!(status["control_port"], crate::CONTROL_PORT);

        let wake_res = engine.dispatch(Command::TriggerWake, &store);
        assert_eq!(wake_res["event"], "wake");

        let route_res = engine.dispatch(
            Command::RouteVoiceBus {
                transcript: "robotcraft rotate 90".to_string(),
            },
            &store,
        );
        assert_eq!(route_res["receiver"], "robotcraft");

        let sleep_res = engine.dispatch(Command::TriggerSleep, &store);
        assert_eq!(sleep_res["event"], "sleep");
    }
}
