# SpeechCraft

> Local-First Acoustic Studio, Real-Time Wake Word Coordinator & Neural Voice Command Bus.

SpeechCraft delivers low-latency, zero-panic, pure-Rust voice capabilities to the ArtCraft fleet, uniting wake word detection, ambient speech recognition, prosody shaping, and broadcast audio mastering on Port **14132**.

---

## Capabilities & Architecture

- **Acoustic Wake Word Engine**: Offline keyword detection (`wakeywakey`, `hey_jarvis`, `alexa`, `hey_mycroft`), VAD energy gating, spoken greetings, and sleep triggers.
- **Fleet Voice Command Bus**: Direct native routing to fleet-agent (`10996`) for entity execution (`fritz`, `codercraft`, `robotcraft`, `calibre`, `plexy`), with spoken feedback and audible timers.
- **Prosody & Phoneme Sculptor**: Interactive sliders for Pitch Semitones, Speech Rate, Chest Warmth, Breathiness, and Acoustic Gravity.
- **Spoken Literature Mastering**: Paragraph timeline with non-destructive regeneration and EBU R128 (-16 LUFS) broadcast loudness mastering.
- **7-Locale Multilingual Support**: English, German, French, Spanish, Japanese, Korean, and Chinese.

## Getting Started

```powershell
# Run all workspace tests and MCP smoke verification
.\start.ps1 -Smoke

# Start the native GPU Desktop Studio
.\start.ps1

# Start the FastMCP stdio server (Port 14132)
.\start.ps1 -Headless

# Synthesize speech directly via CLI
cargo run -p speechcraft-mcp -- --synthesize "Hello fleet"

# Enumerate physical microphone input devices
cargo run -p speechcraft-mcp -- --list-devices
```

## FastMCP Tools (Port 14132)

SpeechCraft exposes standard MCP 2024-11-05 JSON-RPC tools over stdio:

- `speechcraft_status`: Get studio status, active wake word, sensitivity, and control port.
- `speechcraft_list_audio_devices`: Enumerate physical input microphones and audio interfaces.
- `speechcraft_configure_wake`: Configure keyword (`wakeywakey`, `hey_jarvis`, `alexa`, `hey_mycroft`), sensitivity, and greeting.
- `speechcraft_route_intent`: Route natural voice commands to fleet agent bus (`10996`).
- `speechcraft_synthesize`: Synthesize vocal audio with prosody shaping and optional host speaker playback.
- `speechcraft_export_wav`: Generate broadcast-mastered EBU R128 (-16 LUFS) WAV byte buffer.
- `speechcraft_verify_voiceprint`: Authenticate 16kHz audio against enrolled biometric voiceprints.
- `speechcraft_list_voiceprints`: List registered voiceprints and 32-dim biometric embeddings.
- `speechcraft_timeline_manage`: Manage spoken literature multitrack timeline paragraphs.

## Lineage — the Storytold craft-app pattern

Storytold ArtCraft ([storytold/artcraft](https://github.com/storytold/artcraft) — "the IDE for artists") was the starting point. The 16 craft apps in the Storytold stable each ship one binary that is both the human CLI and the agent MCP server (stdio), headless-first, with `run.json` + report artifacts. That modus operandi and harness is what this repo implements; the code here is independent, the pattern is shared.
