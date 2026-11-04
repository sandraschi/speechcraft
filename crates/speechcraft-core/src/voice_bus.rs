//! SpeechCraft Fleet Voice Command Bus Integration.
//!
//! Handles routing `SpeechIntent` payloads to fleet-agent (`10996`),
//! tracking audible timers, and managing spoken voice responses.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use crate::models::{SpeechIntent, VoiceBusConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudibleTimer {
    pub timer_id: String,
    pub label: String,
    pub target_epoch_sec: u64,
    pub is_triggered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceBusRouteResult {
    pub success: bool,
    pub receiver: String,
    pub reply_spoken_text: String,
    pub error_msg: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VoiceBusManager {
    pub config: VoiceBusConfig,
    pub routed_intents: Vec<SpeechIntent>,
    pub active_timers: Vec<AudibleTimer>,
    pub last_reply: Option<String>,
}

impl Default for VoiceBusManager {
    fn default() -> Self {
        Self {
            config: VoiceBusConfig::default(),
            routed_intents: Vec::new(),
            active_timers: Vec::new(),
            last_reply: None,
        }
    }
}

impl VoiceBusManager {
    #[must_use]
    pub fn new(config: VoiceBusConfig) -> Self {
        Self {
            config,
            routed_intents: Vec::new(),
            active_timers: Vec::new(),
            last_reply: None,
        }
    }

    /// Dispatches a speech intent to the appropriate fleet receiver.
    pub fn route_intent(&mut self, intent: SpeechIntent) -> VoiceBusRouteResult {
        let receiver = intent.receiver_entity.clone();
        let cmd = intent.action_command.clone();

        // Check if intent is a timer command
        if cmd.starts_with("set timer") || cmd.starts_with("timer") {
            let timer = AudibleTimer {
                timer_id: format!("T-{}", intent.timestamp_epoch),
                label: cmd.clone(),
                target_epoch_sec: intent.timestamp_epoch.saturating_add(1200), // default 20m
                is_triggered: false,
            };
            self.active_timers.push(timer);
            let reply = format!("Timer set for {cmd}.");
            self.last_reply = Some(reply.clone());
            self.routed_intents.push(intent);
            return VoiceBusRouteResult {
                success: true,
                receiver,
                reply_spoken_text: reply,
                error_msg: None,
            };
        }

        // Standard fleet receiver response simulation
        let reply = match receiver.as_str() {
            "fritz" => format!("OK, Fritz executed: {cmd}"),
            "codercraft" => format!("CoderCraft active buffer patched: {cmd}"),
            "robotcraft" => format!("RobotCraft arm moving: {cmd}"),
            "calibre" => format!("Calibre library searching: {cmd}"),
            "plexy" => format!("Plex media player started: {cmd}"),
            _ => format!("Routed to {receiver}: {cmd}"),
        };

        self.last_reply = Some(reply.clone());
        self.routed_intents.push(intent);

        VoiceBusRouteResult {
            success: true,
            receiver,
            reply_spoken_text: reply,
            error_msg: None,
        }
    }

    /// Checks if any active audible timers have expired.
    pub fn check_timers(&mut self, current_epoch_sec: u64) -> Vec<String> {
        let mut announcements = Vec::new();
        for timer in &mut self.active_timers {
            if !timer.is_triggered && current_epoch_sec >= timer.target_epoch_sec {
                timer.is_triggered = true;
                announcements.push(format!("Timer done: {}", timer.label));
            }
        }
        announcements
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_bus_routing() {
        let mut bus = VoiceBusManager::default();
        let intent = SpeechIntent {
            intent_id: "INT-1".to_string(),
            receiver_entity: "fritz".to_string(),
            raw_transcript: "fritz gpu status".to_string(),
            action_command: "gpu status".to_string(),
            confidence: 0.98,
            timestamp_epoch: 1000,
        };

        let res = bus.route_intent(intent);
        assert!(res.success);
        assert!(res.reply_spoken_text.contains("Fritz executed: gpu status"));
        assert_eq!(bus.routed_intents.len(), 1);
    }

    #[test]
    fn test_timer_announcement() {
        let mut bus = VoiceBusManager::default();
        let intent = SpeechIntent {
            intent_id: "INT-T".to_string(),
            receiver_entity: "fritz".to_string(),
            raw_transcript: "set timer 10m".to_string(),
            action_command: "set timer 10m".to_string(),
            confidence: 0.95,
            timestamp_epoch: 1000,
        };

        let _ = bus.route_intent(intent);
        assert_eq!(bus.active_timers.len(), 1);

        let expired = bus.check_timers(3000);
        assert_eq!(expired.len(), 1);
        assert!(expired[0].contains("Timer done: set timer 10m"));
    }
}
