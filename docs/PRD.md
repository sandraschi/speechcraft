# SpeechCraft — Product Requirements Document (PRD)

**Document Identifier:** `CRAFT-PRD-2026-SPEECH-001`  
**Version:** 1.0.0  
**Status:** Active  
**Domain:** Acoustic Studio, Real-Time Audio Capture, Voiceprint Identification & Voice Bus  
**Standard Port:** 14132  
**Parent Standard:** [`RUST_ARTCRAFT_STANDARD.md`](../../mcp-central-docs/standards/RUST_ARTCRAFT_STANDARD.md)  
**Menu Bar Standard:** [`CRAFT_MENU_BAR_STANDARD.md`](../../mcp-central-docs/standards/rust-artcraft/CRAFT_MENU_BAR_STANDARD.md)  

---

## 1. Executive Summary & Problem Statement

Voice assistant platforms (Alexa, Siri, Google Assistant) stream raw microphone audio to surveillance clouds, introduce multi-second round-trip network latencies, and break when network connectivity is lost.

**The SpeechCraft Solution:**
SpeechCraft is a local-first acoustic studio and voice-command bus coordinator written in 100% safe Rust:
- Real-time `cpal` hardware microphone capture with deterministic linear resampling.
- Keyword template detector and acoustic spectral band analyzer.
- 16-D speaker voiceprint embedding with Cosine similarity verification.
- Local voice intent dispatcher broadcasting events across the fleet voice bus (`POST /api/voice/intent`).
- Local STT/TTS sidecar integration (Whisper / Kokoro) with configurable prosody profiles.

---

## 2. Core Functional Requirements

1. **Acoustic Signal Processing (`speechcraft-dsp`):**
   - High-throughput linear audio resampler.
   - Spectral band analyzer and keyword pattern matching.
   - 16-D speaker voiceprint enrollment and recognition.
2. **Voice Bus Dispatcher (`speechcraft-core`):**
   - Fleet voice command intent normalization.
   - STT / TTS endpoint configuration and prosody parameters.
3. **Universal Command Bus (`Command::dispatch`):**
   - 100% of CLI, MCP, and UI actions dispatch through `speechcraft-core::dispatch`.

---

## 3. UI/UX Architecture & Menu Bar Standard

In accordance with [`CRAFT_MENU_BAR_STANDARD.md`](../../mcp-central-docs/standards/rust-artcraft/CRAFT_MENU_BAR_STANDARD.md):
- **Pattern 2 (Viewport-Centric Acoustic Workstation):** Persistent acoustic spectral waterfall viewport and live microphone audio meter with voiceprint identification HUD. Top quick button row acts as accelerators into the `View` menu. To add a new voice pipeline or audio filter:
  1. Register a visibility toggle in the `View` or `Window` menu.
  2. Add an entry in the sidebar Audio / Voiceprint toolbox.
  3. Wire the action directly into the universal `Command::dispatch` bus.
- **Top Menu Bar Hierarchy:**
  - `Session`: Start / Stop Capture (`Ctrl+Space`), Load WAV Recording (`Ctrl+O`), Open Recent ▸, Save Session (`Ctrl+S`), Export Audio / WAV (`Ctrl+E`), Quit (`Ctrl+Q`).
  - `Edit`: Undo (`Ctrl+Z`), Redo (`Ctrl+Y`), Cut, Copy, Paste, Select All, Preferences (`Ctrl+,`).
  - `View`: Spectral Waterfall Viewport, Live Audio VU Meters, Voiceprint Enrollment Grid, Intent Dispatch Log, STT/TTS Telemetry, Theme (`Dark`/`Light`).
  - `Insert`: Enroll Speaker Voiceprint, Add Wake-Word Keyword, Insert Prosody Preset, Add Audio Filter.
  - `Modify`: Adjust Gain / Threshold, Tune Cosine Similarity Ceiling, Edit Intent Mapping.
  - `Commands`: Run Keyword Sweep, Match Speaker Voiceprint, Broadcast Voice Intent, Test TTS Voice.
  - `Window`: Reset Layout, Acoustic Viewport, Voiceprint Inspector, Telemetry Terminal.
  - `Help` (Mandatory 5-Dialog Suite): `Command Palette...` (`Ctrl+K`), `Keyboard Shortcuts...` (`F1`), `Project Files & Docs...`, `Licenses & Notices...`, `About SpeechCraft`.
- **Behavioral Mechanics:** Smooth `switch_on_hover` between top-level menus, zero-unsafe immediate-mode rendering, and headless `--smoke` parity.
