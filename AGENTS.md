# SpeechCraft — AGENTS.md

**Purpose**: Sovereign acoustic studio, wake word coordinator, and neural voice command bus for the ArtCraft ecosystem.

## Fleet Context
- Sovereign zero-panic Rust architecture (`crates/speechcraft-core`, `crates/speechcraft-dsp`, `crates/speechcraft-app`, `crates/speechcraft-mcp`).
- Control port: **14132** (registered in `mcp-central-docs/operations/WEBAPP_PORTS.md`).
- Fleet voice command bus router ingress: `http://127.0.0.1:10996/api/voice/intent`.
- Wake words: `wakeywakey`, `hey_jarvis`, `alexa`, `hey_mycroft`.
- Fleet-standard 7-locale multilingual i18n (`En`, `De`, `Fr`, `Es`, `Ja`, `Ko`, `Zh`).
- Backed by DatabaseCraft store for voiceprints, audio clips, and intent telemetry.

## Commands
```powershell
# Run smoke tests
cargo test --workspace

# Start desktop studio GUI
cargo run -p speechcraft-app

# Start FastMCP server
cargo run -p speechcraft-mcp -- --control 14132
```
