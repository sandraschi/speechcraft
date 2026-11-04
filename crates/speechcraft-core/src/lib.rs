//! SpeechCraft Core Library & Neural Voice Bus.
//!
//! ArtCraft Sovereign Speech Studio:
//! - Subsumes openWakeWord with zero-cloud acoustic keyword coordinator
//! - Native Voice Command Bus transmitter and receiver for the fleet (`fritz`, `codercraft`, `robotcraft`)
//! - Prosody and phonetic shaping engine
//! - Spoken literature multitrack timeline with EBU R128 LUFS mastering
//! - Enclave voiceprint biometric vault
//! - Multi-lingual i18n support across 7 fleet locales
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

pub mod dispatch;
pub mod i18n;
pub mod models;
pub mod voice_bus;
pub mod wake;

pub use dispatch::{Command, SpeechCraftEngine};
pub use models::*;

pub const CONTROL_PORT: u16 = 14132;
pub const APP_VERSION: &str = "0.1.0";
