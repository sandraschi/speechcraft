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
            wake_greeting: Some("At your service, mistress.".to_string()),
            sleep_keyword: Some("sleepsleep".to_string()),
            listen_command_seconds: Some(5),
        },
        &store,
    );
    assert_eq!(cfg_res["wake_keyword"], "wakeywakey");
    assert_eq!(cfg_res["greeting"], "At your service, mistress.");

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
}
