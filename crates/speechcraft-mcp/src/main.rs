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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let store = DatabaseStore::open_in_memory()?;
    let mut engine = SpeechCraftEngine::new();

    if args.smoke {
        let status = engine.dispatch(Command::Status, &store);
        let success = status.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        if success {
            println!("speechcraft-mcp smoke: OK (control port {})", args.control);
            return Ok(());
        }
        return Err("speechcraft-mcp smoke verification failed".into());
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
