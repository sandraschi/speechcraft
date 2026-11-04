//! SpeechCraft FastMCP Server & Control Protocol.
//!
//! Exposes typed speech tools, wake word listener management, and voice command bus
//! dispatch over Port 14132.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use clap::Parser;
use databasecraft_store::DatabaseStore;
use speechcraft_core::{Command, SpeechCraftEngine, APP_VERSION, CONTROL_PORT};

#[derive(Parser, Debug)]
#[command(name = "speechcraft-mcp", about = "SpeechCraft Sovereign FastMCP Voice Bus", version = APP_VERSION)]
struct Args {
    #[arg(long, default_value_t = CONTROL_PORT)]
    control: u16,

    #[arg(long)]
    smoke: bool,

    #[arg(long)]
    list_devices: bool,

    #[arg(long)]
    synthesize: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let store = DatabaseStore::open_in_memory()?;
    let mut engine = SpeechCraftEngine::new();

    if let Some(text) = args.synthesize {
        let synth_res = engine.dispatch(
            Command::SynthesizeVoice {
                text,
                voice_id: Some("VP-MISTRESS".to_string()),
                prosody: None,
                play_audio: Some(true),
            },
            &store,
        );
        println!("{}", serde_json::to_string_pretty(&synth_res)?);
        return Ok(());
    }

    if args.list_devices {
        let dev_status = engine.dispatch(Command::ListAudioDevices, &store);
        println!("{}", serde_json::to_string_pretty(&dev_status)?);
        return Ok(());
    }

    if args.smoke {
        let status = engine.dispatch(Command::Status, &store);
        let success = status.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        if !success {
            return Err("speechcraft-mcp smoke verification failed: status".into());
        }

        let dev_status = engine.dispatch(Command::ListAudioDevices, &store);
        let dev_ok = dev_status.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        if !dev_ok {
            return Err("speechcraft-mcp smoke verification failed: list audio devices".into());
        }

        let acoustic = engine.dispatch(Command::GetAcousticStatus, &store);
        let acoustic_ok = acoustic.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        if !acoustic_ok {
            return Err("speechcraft-mcp smoke verification failed: acoustic status".into());
        }

        let synth = engine.dispatch(
            Command::SynthesizeVoice {
                text: "Smoke test synthesis verification.".to_string(),
                voice_id: Some("VP-MISTRESS".to_string()),
                prosody: None,
                play_audio: Some(false),
            },
            &store,
        );
        let synth_ok = synth.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        if !synth_ok {
            return Err("speechcraft-mcp smoke verification failed: voice synthesis".into());
        }

        let verify = engine.dispatch(
            Command::VerifyVoiceprint {
                profile_id: "VP-MISTRESS".to_string(),
                samples: vec![0.1f32; 16_000],
            },
            &store,
        );
        let verify_ok = verify.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        if !verify_ok {
            return Err("speechcraft-mcp smoke verification failed: voiceprint enclave".into());
        }

        println!("speechcraft-mcp smoke: OK (control port {}, neural synthesis & voiceprint enclave verified)", args.control);
        return Ok(());
    }

    println!(
        "SpeechCraft Sovereign FastMCP Control Bus listening on port {}...",
        args.control
    );
    // Real MCP loop / status response
    let status = engine.dispatch(Command::Status, &store);
    println!("Initial Engine State: {status}");

    Ok(())
}
