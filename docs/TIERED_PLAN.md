# SpeechCraft — Tiered Execution Plan

**Parent Standard:** [`RUST_ARTCRAFT_STANDARD.md`](../../mcp-central-docs/standards/RUST_ARTCRAFT_STANDARD.md)  
**Menu Bar Standard:** [`CRAFT_MENU_BAR_STANDARD.md`](../../mcp-central-docs/standards/rust-artcraft/CRAFT_MENU_BAR_STANDARD.md)  
**Architecture Model:** 4-Tier ArtCraft Incremental Delivery  
**Status:** Active  

---

## Architecture Flow

```
Tier 1: Core Engine ──> Tier 2: MCP / CLI ──> Tier 3: Desktop UI ──> Tier 4: Menu Bar, Help & Packaging
(speechcraft-dsp/core) (speechcraft-mcp)     (speechcraft-app)     (Hardened Workstation)
```

---

## Tier Specifications

### Tier 1 — Core Acoustic DSP & Intent Engine (`speechcraft-dsp`, `speechcraft-core`)
- [x] L1 `Command::dispatch` universal voice execution bus.
- [x] Live `cpal` microphone audio capture with linear resampling.
- [x] Spectral band analyzer and keyword pattern detector.
- [x] 16-D speaker voiceprint embedding and Cosine similarity matcher.
- [x] Local voice-intent HTTP broadcaster (`POST /api/voice/intent`).
- [x] `#![forbid(unsafe_code)]` and zero-panic clippy lints.

### Tier 2 — Headless Driver & FastMCP (`speechcraft-mcp`)
- [x] Unified CLI `speechcraft-cli` with `--json` output.
- [x] FastMCP stdio server communicating over JSON-RPC 2.0.
- [x] Deterministic DSP analysis on fixture audio files.
- [x] Headless `--smoke` verification without window initialization.

### Tier 3 — Desktop Workstation UI (`speechcraft-app`)
- [x] Immediate-mode `egui` 0.37 / `wgpu` workstation.
- [x] Live input VU meter, spectral waterfall, and microphone controls.
- [x] Speaker voiceprint enrollment matrix and intent log monitor.

### Tier 4 — Craft Menu Bar Standard, Help Suite & Hardened Verification
- [ ] Universal 8-menu bar (`Session`/`File`, `Edit`, `View`, `Insert`, `Commands`, `Window`, `Help`) with `switch_on_hover`.
- [ ] Mandatory 5-dialog Help suite (`Command Palette...`, `Keyboard Shortcuts...`, `Project Files & Docs...`, `Licenses & Notices...`, `About SpeechCraft`).
- [ ] Fleet UI/UX Pattern 2 adherence: viewport-centric navigation, top button row mapped to View toggles, panel visibility managed via `View` / `Window`.
- [ ] Tofu Guard verification (100% font glyph coverage, zero tofu placeholder boxes).
- [ ] Headless `--smoke` parity validation.
- [ ] Standalone release binary packaging and NSIS installer.
