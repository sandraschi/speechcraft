//! SpeechCraft Core Domain Models.
//!
//! Encompasses wake words, speech intents, voice command bus, STT/TTS providers,
//! prosody shaping, and voiceprint biometric profiles.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WakeKeyword {
    HeyJarvis,
    WakeyWakey,
    Alexa,
    HeyMycroft,
    Custom(String),
}

impl WakeKeyword {
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            WakeKeyword::HeyJarvis => "hey_jarvis",
            WakeKeyword::WakeyWakey => "wakeywakey",
            WakeKeyword::Alexa => "alexa",
            WakeKeyword::HeyMycroft => "hey_mycroft",
            WakeKeyword::Custom(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WakeWordConfig {
    pub keyword: WakeKeyword,
    pub vad_threshold: f32,
    pub sensitivity: f32,
    pub wake_greeting: String,
    pub sleep_keyword: String,
    pub is_listening: bool,
    pub listen_command_seconds: u32,
}

impl Default for WakeWordConfig {
    fn default() -> Self {
        Self {
            keyword: WakeKeyword::WakeyWakey,
            vad_threshold: 0.5,
            sensitivity: 0.5,
            wake_greeting: "Hello, mistress.".to_string(),
            sleep_keyword: "sleepsleep".to_string(),
            is_listening: true,
            listen_command_seconds: 6,
        }
    }
}

/// Dispatched speech intent routed through the fleet voice command bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechIntent {
    pub intent_id: String,
    pub receiver_entity: String, // "fritz", "codercraft", "robotcraft", "opencode", "calibre", "plexy"
    pub raw_transcript: String,
    pub action_command: String,
    pub confidence: f32,
    pub timestamp_epoch: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceBusConfig {
    pub router_url: String, // Default "http://127.0.0.1:10996/api/voice/intent"
    pub default_entity: String, // "fritz"
    pub speak_reply: bool,
    pub auto_start_listener: bool,
}

impl Default for VoiceBusConfig {
    fn default() -> Self {
        Self {
            router_url: "http://127.0.0.1:10996/api/voice/intent".to_string(),
            default_entity: "fritz".to_string(),
            speak_reply: true,
            auto_start_listener: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SttProvider {
    FunAsr,
    WhisperLocal,
    SherpaOnnx,
    GeminiLive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TtsProvider {
    KokoroLocal,
    Qwen3Tts,
    HumeEvi,
    ElevenLabs,
    GeminiFlash,
}

/// Expressive vocal prosody shaping parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocalProsody {
    pub pitch_semitones: f32, // -12.0 .. +12.0
    pub rate_multiplier: f32, // 0.5 .. 2.0
    pub warmth: f32,          // 0.0 .. 1.0 (Low-mid formant boost)
    pub breathiness: f32,     // 0.0 .. 1.0 (Airy high-frequency aspiration)
    pub gravity: f32,         // 0.0 .. 1.0 (Chest resonance)
}

impl Default for VocalProsody {
    fn default() -> Self {
        Self {
            pitch_semitones: 0.0,
            rate_multiplier: 1.0,
            warmth: 0.5,
            breathiness: 0.2,
            gravity: 0.5,
        }
    }
}

/// Spoken literature paragraph timeline item (e.g. from WriterCraft).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpokenTimelineItem {
    pub id: String,
    pub paragraph_idx: usize,
    pub raw_text: String,
    pub phonetic_override: Option<String>,
    pub speaker_voice_id: String,
    pub prosody: VocalProsody,
    pub target_lufs: f32,
    pub audio_duration_ms: u64,
    pub is_rendered: bool,
}

/// Voiceprint biometric profile (local enclave).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceprintProfile {
    pub profile_id: String,
    pub name: String,
    pub gender: String,
    pub accent: String,
    pub sample_count: usize,
    pub similarity_confidence: f32,
    pub embedding: Vec<f32>,
}

/// Persisted studio settings (audio, wake, bus, providers, UI).
/// Stored as JSON in `speechcraft-settings.json` next to the working dir.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechCraftSettings {
    pub input_device_name: Option<String>,
    pub output_device_name: Option<String>,
    pub keyword: String,
    pub sensitivity: f32,
    pub vad_threshold: f32,
    pub wake_greeting: String,
    pub sleep_keyword: String,
    pub listen_command_seconds: u32,
    pub router_url: String,
    pub speak_reply: bool,
    pub stt_provider: SttProvider,
    pub tts_provider: TtsProvider,
    pub locale_code: String,
    pub dark_theme: bool,
}

impl Default for SpeechCraftSettings {
    fn default() -> Self {
        let wake = WakeWordConfig::default();
        let bus = VoiceBusConfig::default();
        Self {
            input_device_name: None,
            output_device_name: None,
            keyword: wake.keyword.as_str().to_string(),
            sensitivity: wake.sensitivity,
            vad_threshold: wake.vad_threshold,
            wake_greeting: wake.wake_greeting,
            sleep_keyword: wake.sleep_keyword,
            listen_command_seconds: wake.listen_command_seconds,
            router_url: bus.router_url,
            speak_reply: bus.speak_reply,
            stt_provider: SttProvider::WhisperLocal,
            tts_provider: TtsProvider::KokoroLocal,
            locale_code: "en".to_string(),
            dark_theme: true,
        }
    }
}

/// Synthesized audio clip metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizedClip {
    pub clip_id: String,
    pub text: String,
    pub voice_id: String,
    pub prosody: VocalProsody,
    pub sample_rate: u32,
    pub duration_ms: u64,
    pub lufs: f32,
    pub pcm_sample_count: usize,
}
