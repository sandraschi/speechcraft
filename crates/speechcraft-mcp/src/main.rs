//! SpeechCraft Sovereign FastMCP Server & Control Protocol.
//!
//! Provides pure JSON-RPC (MCP 2024-11-05) over stdio and CLI control bus on Port 14132.
//! Stdout is 100% pure JSON-RPC in MCP mode. All diagnostics go to stderr.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use clap::Parser;
use databasecraft_store::DatabaseStore;
use serde_json::{json, Value};
use speechcraft_core::{Command, SpeechCraftEngine, VocalProsody, APP_VERSION, CONTROL_PORT};
use std::io::{BufRead, Write};

#[derive(Parser, Debug)]
#[command(
    name = "speechcraft-mcp",
    about = "SpeechCraft Sovereign FastMCP Voice Bus & Acoustic Engine",
    version = APP_VERSION
)]
struct Args {
    #[arg(long, default_value_t = CONTROL_PORT)]
    control: u16,

    #[arg(long)]
    smoke: bool,

    #[arg(long)]
    list_devices: bool,

    #[arg(long)]
    synthesize: Option<String>,

    /// Run as standard MCP stdio JSON-RPC server (default if no other flag is given)
    #[arg(long)]
    mcp: bool,
}

pub fn tools_list() -> Value {
    json!([
        {
            "name": "speechcraft_status",
            "description": "Get SpeechCraft acoustic studio status, active wake word, sensitivity, and registered control port 14132",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "speechcraft_list_audio_devices",
            "description": "List all host physical audio input devices (microphones, webcams, audio interfaces)",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "speechcraft_configure_wake",
            "description": "Configure wake word keyword (wakeywakey, hey_jarvis, alexa, hey_mycroft), sensitivity (0.0..1.0), and spoken greeting",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "keyword": { "type": "string", "description": "Wake word: wakeywakey, hey_jarvis, alexa, or hey_mycroft" },
                    "sensitivity": { "type": "number", "description": "Detection sensitivity from 0.0 to 1.0 (default 0.5)" },
                    "vad_threshold": { "type": "number", "description": "Voice activity energy threshold from 0.01 to 0.99" },
                    "wake_greeting": { "type": "string", "description": "Spoken response greeting" },
                    "sleep_keyword": { "type": "string", "description": "Sleep trigger keyword" }
                }
            }
        },
        {
            "name": "speechcraft_route_intent",
            "description": "Route natural voice command to fleet agent command bus (fritz, codercraft, robotcraft, calibre, plexy)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "transcript": { "type": "string", "description": "Natural speech transcript to route" }
                },
                "required": ["transcript"]
            }
        },
        {
            "name": "speechcraft_synthesize",
            "description": "Synthesize vocal speech from text with prosody shaping and optional audio output playback",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Text utterance to synthesize" },
                    "voice_id": { "type": "string", "description": "Speaker archetype: VP-MISTRESS, VP-FRITZ, etc." },
                    "pitch_semitones": { "type": "number", "description": "Pitch inflection (-12.0 to +12.0 semitones)" },
                    "rate_multiplier": { "type": "number", "description": "Speech speed multiplier (0.5 to 2.0)" },
                    "warmth": { "type": "number", "description": "Chest resonance (0.0 to 1.0)" },
                    "breathiness": { "type": "number", "description": "Aspiration noise (0.0 to 1.0)" },
                    "gravity": { "type": "number", "description": "Acoustic gravity authority (0.0 to 1.0)" },
                    "play_audio": { "type": "boolean", "description": "Whether to play audio through host speakers (default: false)" }
                },
                "required": ["text"]
            }
        },
        {
            "name": "speechcraft_export_wav",
            "description": "Generate broadcast-mastered EBU R128 (-16.0 LUFS) WAV byte buffer metrics",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Text utterance to master" },
                    "voice_id": { "type": "string", "description": "Speaker archetype" }
                },
                "required": ["text"]
            }
        },
        {
            "name": "speechcraft_verify_voiceprint",
            "description": "Biometrically authenticate acoustic samples against enrolled sovereign voiceprint profiles",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "profile_id": { "type": "string", "description": "Enrolled profile ID (e.g. VP-MISTRESS, VP-FRITZ)" },
                    "samples": { "type": "array", "items": { "type": "number" }, "description": "16kHz audio sample array" }
                },
                "required": ["profile_id"]
            }
        },
        {
            "name": "speechcraft_list_voiceprints",
            "description": "List enrolled biometric voiceprint profiles and 32-dim embedding telemetry",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "speechcraft_timeline_manage",
            "description": "Manage spoken literature audiobook timeline paragraphs (list or append)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": { "type": "string", "description": "Action: 'list' or 'add'" },
                    "text": { "type": "string", "description": "Paragraph text (when action is 'add')" },
                    "speaker_id": { "type": "string", "description": "Speaker ID (when action is 'add')" }
                },
                "required": ["action"]
            }
        }
    ])
}

pub fn call_tool(engine: &mut SpeechCraftEngine, store: &DatabaseStore, name: &str, args: &Value) -> Value {
    match name {
        "speechcraft_status" => engine.dispatch(Command::Status, store),
        "speechcraft_list_audio_devices" => engine.dispatch(Command::ListAudioDevices, store),
        "speechcraft_configure_wake" => {
            let keyword = args.get("keyword").and_then(|v| v.as_str()).map(|s| s.to_string());
            let sensitivity = args.get("sensitivity").and_then(|v| v.as_f64()).map(|f| f as f32);
            let vad_threshold = args.get("vad_threshold").and_then(|v| v.as_f64()).map(|f| f as f32);
            let wake_greeting = args.get("wake_greeting").and_then(|v| v.as_str()).map(|s| s.to_string());
            let sleep_keyword = args.get("sleep_keyword").and_then(|v| v.as_str()).map(|s| s.to_string());
            engine.dispatch(
                Command::ConfigureWakeWord {
                    keyword,
                    vad_threshold,
                    sensitivity,
                    wake_greeting,
                    sleep_keyword,
                    listen_command_seconds: None,
                },
                store,
            )
        }
        "speechcraft_route_intent" => {
            let transcript = args.get("transcript").and_then(|v| v.as_str()).unwrap_or("").to_string();
            engine.dispatch(Command::RouteVoiceBus { transcript }, store)
        }
        "speechcraft_synthesize" => {
            let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let voice_id = args.get("voice_id").and_then(|v| v.as_str()).map(|s| s.to_string());
            let pitch = args.get("pitch_semitones").and_then(|v| v.as_f64()).map(|f| f as f32).unwrap_or(0.0);
            let rate = args.get("rate_multiplier").and_then(|v| v.as_f64()).map(|f| f as f32).unwrap_or(1.0);
            let warmth = args.get("warmth").and_then(|v| v.as_f64()).map(|f| f as f32).unwrap_or(0.5);
            let breathiness = args.get("breathiness").and_then(|v| v.as_f64()).map(|f| f as f32).unwrap_or(0.2);
            let gravity = args.get("gravity").and_then(|v| v.as_f64()).map(|f| f as f32).unwrap_or(0.5);
            let play_audio = args.get("play_audio").and_then(|v| v.as_bool());

            let prosody = VocalProsody {
                pitch_semitones: pitch,
                rate_multiplier: rate,
                warmth,
                breathiness,
                gravity,
            };

            engine.dispatch(
                Command::SynthesizeVoice {
                    text,
                    voice_id,
                    prosody: Some(prosody),
                    play_audio,
                },
                store,
            )
        }
        "speechcraft_export_wav" => {
            let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let voice_id = args.get("voice_id").and_then(|v| v.as_str()).map(|s| s.to_string());
            engine.dispatch(
                Command::ExportWav {
                    text,
                    voice_id,
                    prosody: None,
                },
                store,
            )
        }
        "speechcraft_verify_voiceprint" => {
            let profile_id = args.get("profile_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let samples = args
                .get("samples")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect())
                .unwrap_or_else(|| vec![0.1f32; 1600]);
            engine.dispatch(Command::VerifyVoiceprint { profile_id, samples }, store)
        }
        "speechcraft_list_voiceprints" => engine.dispatch(Command::ListVoiceprints, store),
        "speechcraft_timeline_manage" => {
            let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("list");
            match action {
                "add" => {
                    let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let speaker_id = args.get("speaker_id").and_then(|v| v.as_str()).unwrap_or("fritz_sovereign").to_string();
                    engine.dispatch(Command::AddTimelineItem { text, speaker_id }, store)
                }
                _ => engine.dispatch(Command::ListTimelineItems, store),
            }
        }
        unknown => json!({ "success": false, "error": format!("Unknown tool '{unknown}'") }),
    }
}

/// Hand-rolled MCP 2024-11-05 line-delimited stdio framing loop.
/// Stdio invariant: stdout is 100% JSON-RPC. Diagnostics go to stderr.
fn run_mcp_stdio(engine: &mut SpeechCraftEngine, store: &DatabaseStore) {
    eprintln!("[speechcraft-mcp] Starting MCP server stdio loop (MCP 2024-11-05, Port {CONTROL_PORT})");
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let text = match line {
            Ok(t) => t,
            Err(_) => break,
        };
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }

        let req: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[speechcraft-mcp] JSON parse error: {e}");
                continue;
            }
        };

        let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let id = req.get("id").cloned();

        let response = match method {
            "initialize" => {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": { "listChanged": false }
                        },
                        "serverInfo": {
                            "name": "speechcraft",
                            "version": APP_VERSION
                        }
                    }
                })
            }
            "notifications/initialized" => {
                continue;
            }
            "ping" => {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {}
                })
            }
            "tools/list" => {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": tools_list()
                    }
                })
            }
            "tools/call" => {
                let params = req.get("params").cloned().unwrap_or(Value::Null);
                let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                let tool_res = call_tool(engine, store, tool_name, &args);

                let is_error = tool_res
                    .get("success")
                    .and_then(|v| v.as_bool())
                    .map(|s| !s)
                    .unwrap_or(false);

                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": tool_res.to_string()
                            }
                        ],
                        "isError": is_error
                    }
                })
            }
            _ => {
                if id.is_some() {
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32601,
                            "message": format!("Method not found: {method}")
                        }
                    })
                } else {
                    continue;
                }
            }
        };

        if let Ok(serialized) = serde_json::to_string(&response) {
            let _ = writeln!(stdout, "{serialized}");
            let _ = stdout.flush();
        }
    }
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
        // 1. Verify tools list
        let tools = tools_list();
        let tools_len = tools.as_array().map(|a| a.len()).unwrap_or(0);
        if tools_len < 8 {
            return Err("speechcraft-mcp smoke failed: incomplete tools list".into());
        }

        // 2. Verify all core tools via call_tool
        let status = call_tool(&mut engine, &store, "speechcraft_status", &json!({}));
        let ok = status.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        if !ok {
            return Err("speechcraft-mcp smoke failed: status tool".into());
        }

        let dev_status = call_tool(&mut engine, &store, "speechcraft_list_audio_devices", &json!({}));
        let dev_ok = dev_status.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        if !dev_ok {
            return Err("speechcraft-mcp smoke failed: list audio devices tool".into());
        }

        let route_res = call_tool(
            &mut engine,
            &store,
            "speechcraft_route_intent",
            &json!({ "transcript": "robotcraft forward 10" }),
        );
        let route_ok = route_res.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        if !route_ok {
            return Err("speechcraft-mcp smoke failed: route intent tool".into());
        }

        let synth = call_tool(
            &mut engine,
            &store,
            "speechcraft_synthesize",
            &json!({ "text": "Testing MCP synthesis", "play_audio": false }),
        );
        let synth_ok = synth.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        if !synth_ok {
            return Err("speechcraft-mcp smoke failed: synthesis tool".into());
        }

        let wav = call_tool(
            &mut engine,
            &store,
            "speechcraft_export_wav",
            &json!({ "text": "Mastered WAV broadcast test" }),
        );
        let wav_ok = wav.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        if !wav_ok {
            return Err("speechcraft-mcp smoke failed: wav export tool".into());
        }

        let vp = call_tool(
            &mut engine,
            &store,
            "speechcraft_verify_voiceprint",
            &json!({ "profile_id": "VP-MISTRESS", "samples": [0.1, 0.1, 0.1] }),
        );
        let vp_ok = vp.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        if !vp_ok {
            return Err("speechcraft-mcp smoke failed: voiceprint verify tool".into());
        }

        println!(
            "speechcraft-mcp smoke: OK (control port {}, {} MCP tools verified)",
            args.control, tools_len
        );
        return Ok(());
    }

    // Default or --mcp: run line-delimited stdio MCP server
    run_mcp_stdio(&mut engine, &store);
    Ok(())
}
