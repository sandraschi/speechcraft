# SpeechCraft

> Sovereign Acoustic Studio, Real-Time Wake Word Coordinator & Neural Voice Command Bus.

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
# Run all workspace tests
cargo test --workspace

# Start the native GPU Desktop Studio
cargo run -p speechcraft-app

# Start the FastMCP server
cargo run -p speechcraft-mcp -- --control 14132
```
