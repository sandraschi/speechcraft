# SpeechCraft — AGENTS.md

**Purpose**: Local-first acoustic studio, wake word coordinator, and neural voice command bus for the ArtCraft ecosystem.

## Fleet Context
- Local-first zero-panic Rust architecture (`crates/speechcraft-core`, `crates/speechcraft-dsp`, `crates/speechcraft-app`, `crates/speechcraft-mcp`).
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


## Documentation preservation and hooks

On 2026-11-20 a README "standardization" sweep across the fleet deleted project descriptions,
status sections and user warnings in nine repos. Full rules:
`mcp-central-docs/standards/CONTENT_PRESERVATION_STANDARD.md`.

- Applying any standard or template restructures; it never deletes. Move content (within the file
  or to `docs/` with a link), do not drop it. Never remove what the project is, what it is based on,
  its status (what works, what does not, what is not built) or warnings to users.
- Remove content only when Sandra explicitly asks for that removal, in this repo.
- Never add claims you have not verified (release downloads, versions, test counts, features).
- Only standards with `status: active` may be applied across repos. A change across several repos is
  done on one repo first and approved by Sandra; its commits name the tool and carry a `Sweep-Id:` line.
- `.githooks/commit-msg` blocks a commit that deletes a Markdown file, removes 20% of its lines
  (15 or more) or removes 3 or more headings. A removal Sandra asked for passes with
  `Docs-Removal-Approved: <who and why>` in the message. Enable once per clone:
  `git config core.hooksPath .githooks`. Never use `--no-verify`; if a hook blocks you, stop and ask.
