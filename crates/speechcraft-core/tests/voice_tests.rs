use databasecraft_store::DatabaseStore;
use speechcraft_core::{Command, SpeechCraftEngine, CONTROL_PORT};

#[test]
fn test_speechcraft_control_port_and_status() {
    let store = DatabaseStore::open_in_memory().unwrap();
    let mut engine = SpeechCraftEngine::new();

    let res = engine.dispatch(Command::Status, &store);
    assert_eq!(res["success"], true);
    assert_eq!(res["app"], "speechcraft");
    assert_eq!(res["control_port"], CONTROL_PORT);
}

#[test]
fn test_speechcraft_wake_word_and_routing() {
    let store = DatabaseStore::open_in_memory().unwrap();
    let mut engine = SpeechCraftEngine::new();

    // 1. Configure wake word
    let cfg_res = engine.dispatch(
        Command::ConfigureWakeWord {
            keyword: Some("wakeywakey".to_string()),
            vad_threshold: Some(0.6),
            sensitivity: Some(0.5),
            wake_greeting: Some("At your service, mistress.".to_string()),
            sleep_keyword: Some("sleepsleep".to_string()),
            listen_command_seconds: Some(5),
        },
        &store,
    );
    assert_eq!(cfg_res["wake_keyword"], "wakeywakey");
    assert_eq!(cfg_res["sensitivity"], 0.5);
    assert_eq!(cfg_res["greeting"], "At your service, mistress.");

    // 1b. Test Audio Devices and Acoustic Status
    let dev_res = engine.dispatch(Command::ListAudioDevices, &store);
    assert_eq!(dev_res["success"], true);

    let status_res = engine.dispatch(Command::GetAcousticStatus, &store);
    assert_eq!(status_res["success"], true);
    assert_eq!(status_res["keyword"], "wakeywakey");

    // 2. Trigger Wake
    let wake_res = engine.dispatch(Command::TriggerWake, &store);
    assert_eq!(wake_res["event"], "wake");

    // 3. Route speech intent through voice bus
    let route_res = engine.dispatch(
        Command::RouteVoiceBus {
            transcript: "robotcraft patrol perimeter".to_string(),
        },
        &store,
    );
    assert_eq!(route_res["success"], true);
    assert_eq!(route_res["receiver"], "robotcraft");

    // 4. Trigger Sleep
    let sleep_res = engine.dispatch(Command::TriggerSleep, &store);
    assert_eq!(sleep_res["event"], "sleep");
}

#[test]
fn test_speechcraft_spoken_timeline_and_voiceprints() {
    let store = DatabaseStore::open_in_memory().unwrap();
    let mut engine = SpeechCraftEngine::new();

    let list_tl = engine.dispatch(Command::ListTimelineItems, &store);
    assert!(list_tl["timeline"].as_array().unwrap().len() >= 1);

    let add_res = engine.dispatch(
        Command::AddTimelineItem {
            text: "Chapter 1: The Sovereign Echoes.".to_string(),
            speaker_id: "fritz_sovereign".to_string(),
        },
        &store,
    );
    assert_eq!(add_res["success"], true);

    let vp_res = engine.dispatch(Command::ListVoiceprints, &store);
    assert!(vp_res["voiceprints"].as_array().unwrap().len() >= 2);

    // 2. Synthesize voice with prosody
    let synth_res = engine.dispatch(
        Command::SynthesizeVoice {
            text: "Sovereign voice synthesis online.".to_string(),
            voice_id: Some("VP-MISTRESS".to_string()),
            prosody: None,
            play_audio: Some(false),
        },
        &store,
    );
    assert_eq!(synth_res["success"], true);
    assert!(synth_res["duration_ms"].as_u64().unwrap() > 400);

    // 3. Export WAV
    let wav_res = engine.dispatch(
        Command::ExportWav {
            text: "Testing broadcast master.".to_string(),
            voice_id: Some("VP-FRITZ".to_string()),
            prosody: None,
        },
        &store,
    );
    assert_eq!(wav_res["success"], true);
    assert!(wav_res["byte_count"].as_u64().unwrap() > 44);

    // 4. Voiceprint Biometric Verification
    let samples = vec![0.1f32; 16_000];
    let verify_res = engine.dispatch(
        Command::VerifyVoiceprint {
            profile_id: "VP-MISTRESS".to_string(),
            samples,
        },
        &store,
    );
    assert_eq!(verify_res["success"], true);
    assert!(verify_res["similarity"].as_f64().unwrap() > 0.0);
}
