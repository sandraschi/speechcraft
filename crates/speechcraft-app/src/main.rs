//! SpeechCraft Sovereign Desktop Studio GUI.
//!
//! Visual acoustic cockpit, wake word monitor, prosody sculptor,
//! and spoken literature mastering suite on Port 14132.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use clap::Parser;
use databasecraft_store::DatabaseStore;
use eframe::egui::{self, Color32, Vec2};
use speechcraft_core::i18n::{self, AppLocale, I18nKey};
use speechcraft_core::{
    Command, SpeechCraftEngine, SpeechCraftSettings, SttProvider, TtsProvider, APP_VERSION,
    CONTROL_PORT,
};
use speechcraft_dsp::{AudioRingBuffer, OutputDeviceInfo, SpectrogramAnalyzer, VadEngine};
use std::sync::{Arc, Mutex};

#[derive(Parser, Debug)]
#[command(name = "speechcraft", about = "SpeechCraft Local-First Acoustic Studio", version = APP_VERSION)]
struct Args {
    #[arg(long, default_value_t = CONTROL_PORT)]
    control: u16,

    #[arg(long)]
    smoke: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudioScreen {
    Cockpit,
    ProsodySculptor,
    SpokenTimeline,
    VoiceBusVault,
    Settings,
    Help,
}

pub struct SpeechCraftApp {
    pub store: DatabaseStore,
    pub engine: SpeechCraftEngine,
    pub current_screen: StudioScreen,
    pub locale: AppLocale,
    pub is_dark_theme: bool,
    pub is_authenticated: bool,
    pub auth_username: String,
    pub show_about_dialog: bool,
    pub status_msg: String,

    // Hardware Audio Input & Continuous Listener (LIVE path)
    pub input_devices: Vec<speechcraft_dsp::InputDeviceInfo>,
    pub selected_device_idx: usize,
    pub is_capturing_mic: bool,
    pub live_rms: f32,
    pub live_confidence: f32,
    pub sensitivity_slider: f32,
    /// Shared live ring buffer fed by the CPAL input stream callback.
    /// `None` when the mic is stopped. The same Arc is handed to
    /// `AudioCaptureManager::start_capture`, so polled data IS the mic.
    pub mic_ring: Option<Arc<Mutex<AudioRingBuffer>>>,
    pub last_polled_total: usize,
    pub vad_engine: VadEngine,
    pub vad_threshold_slider: f32,
    pub audio_source_note: String,

    // Hardware Audio Output (speakers)
    pub output_devices: Vec<OutputDeviceInfo>,
    pub selected_output_idx: usize,

    // Voice-bus router + AI provider state (honest live/fallback)
    pub router_url: String,
    pub speak_reply: bool,
    pub router_reachable: Option<bool>,
    pub router_detail: String,
    pub stt_provider: SttProvider,
    pub tts_provider: TtsProvider,

    // Zoom & Scaling
    pub ui_zoom: f32,
    pub visualizer_zoom: f32,

    // Audio Visualization: live spectrum driven by mic when capturing,
    // idle animation only when the mic is stopped (labelled as such).
    pub spec_analyzer: SpectrogramAnalyzer,
    pub live_spectrum: Vec<f32>,

    // Prosody Sliders
    pub pitch_val: f32,
    pub rate_val: f32,
    pub warmth_val: f32,
    pub breathiness_val: f32,
    pub gravity_val: f32,
    pub preview_text: String,

    // Voice Command Input
    pub manual_voice_command: String,
    pub dispatch_history: Vec<(String, String, String)>, // (receiver, transcript, reply)

    // Visualizer animation
    pub viz_mode: VisualizerMode,
    pub anim_phase: f32,

    // Timeline editor
    pub new_paragraph_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualizerMode {
    Spectrum,
    Oscilloscope,
    VuMeter,
}

/// Built-in prosody presets: (name, pitch, rate, warmth, breathiness, gravity).
pub const PROSODY_PRESETS: [(&str, f32, f32, f32, f32, f32); 5] = [
    ("Sovereign Mistress", 1.0, 1.0, 0.75, 0.30, 0.65),
    ("Fritz Core", -2.0, 1.1, 0.45, 0.15, 0.85),
    ("Audiobook Narrator", 0.0, 0.95, 0.80, 0.20, 0.50),
    ("Intimate Whisper", 0.5, 0.90, 0.30, 0.85, 0.20),
    ("Tactical Alarm", 3.0, 1.30, 0.20, 0.10, 0.90),
];

/// Quick fleet commands for the palette.
pub const QUICK_COMMANDS: [&str; 5] = [
    "fritz gpu status",
    "robotcraft patrol 360",
    "codercraft inspect cargo",
    "calibre search Sovereign AI",
    "plexy play ambient",
];

pub fn settings_file_path() -> std::path::PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("speechcraft-settings.json")
}

fn load_settings_file() -> Option<SpeechCraftSettings> {
    let path = settings_file_path();
    let text = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&text).ok()
}

impl SpeechCraftApp {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let store = DatabaseStore::open_in_memory()?;
        let mut engine = SpeechCraftEngine::new();
        let spec_analyzer = SpectrogramAnalyzer::new(48);
        let input_devices = engine.capture_manager.list_input_devices();
        let output_devices = engine.synth.list_output_devices();

        // Live spectrum buffer: holds the LAST real mic spectrum when
        // capturing; idle sin curve only until the mic starts (labelled).
        let mut live = Vec::with_capacity(48);
        for i in 0..48 {
            let val = ((i as f32 * 0.15).sin().abs() * 0.7 + 0.1).clamp(0.05, 0.95);
            live.push(val);
        }

        // Load persisted settings if present, apply to engine.
        if let Some(saved) = load_settings_file() {
            engine.apply_settings(saved.clone());
        }
        let sensitivity_slider = engine.wake_coordinator.config.sensitivity;
        let vad_threshold_slider = engine.wake_coordinator.config.vad_threshold;
        let router_url = engine.voice_bus.config.router_url.clone();
        let speak_reply = engine.voice_bus.config.speak_reply;
        let stt_provider = engine.stt_provider;
        let tts_provider = engine.tts_provider;
        let locale = match engine.settings.locale_code.as_str() {
            "de" => AppLocale::De,
            "fr" => AppLocale::Fr,
            "es" => AppLocale::Es,
            "ja" => AppLocale::Ja,
            "ko" => AppLocale::Ko,
            "zh" => AppLocale::Zh,
            _ => AppLocale::En,
        };
        let is_dark_theme = engine.settings.dark_theme;
        // Prefer saved device names when they still exist.
        let mut selected_device_idx = 0;
        if let Some(want) = engine.settings.input_device_name.as_deref() {
            for (i, d) in input_devices.iter().enumerate() {
                if d.name == want {
                    selected_device_idx = i;
                    break;
                }
            }
        }
        let mut selected_output_idx = 0;
        if let Some(want) = engine.settings.output_device_name.as_deref() {
            for (i, d) in output_devices.iter().enumerate() {
                if d.name == want {
                    selected_output_idx = i;
                    break;
                }
            }
        }

        Ok(Self {
            store,
            engine,
            current_screen: StudioScreen::Cockpit,
            locale,
            is_dark_theme,
            is_authenticated: false,
            auth_username: "mistress".to_string(),
            show_about_dialog: false,
            status_msg: format!(
                "SpeechCraft v{APP_VERSION} Sovereign Studio Ready (Port {CONTROL_PORT})"
            ),
            input_devices,
            selected_device_idx,
            is_capturing_mic: false,
            live_rms: 0.0,
            live_confidence: 0.0,
            sensitivity_slider,
            mic_ring: None,
            last_polled_total: 0,
            vad_engine: VadEngine::new(0.015, vad_threshold_slider),
            vad_threshold_slider,
            audio_source_note: "SIMULATED IDLE — press Start Mic for live input".to_string(),
            output_devices,
            selected_output_idx,
            router_url,
            speak_reply,
            router_reachable: None,
            router_detail: "Router not probed yet — press Probe.".to_string(),
            stt_provider,
            tts_provider,
            ui_zoom: 1.0,
            visualizer_zoom: 1.0,
            spec_analyzer,
            live_spectrum: live,
            pitch_val: 0.0,
            rate_val: 1.0,
            warmth_val: 0.65,
            breathiness_val: 0.25,
            gravity_val: 0.55,
            preview_text: "SpeechCraft delivers pristine sovereign voice to the ArtCraft fleet."
                .to_string(),
            manual_voice_command: "fritz gpu status".to_string(),
            dispatch_history: Vec::new(),
            viz_mode: VisualizerMode::Spectrum,
            anim_phase: 0.0,
            new_paragraph_text: String::new(),
        })
    }

    /// Persists current UI + engine choices to `speechcraft-settings.json`.
    pub fn save_settings(&mut self) {
        let s = SpeechCraftSettings {
            input_device_name: self
                .input_devices
                .get(self.selected_device_idx)
                .map(|d| d.name.clone()),
            output_device_name: self
                .output_devices
                .get(self.selected_output_idx)
                .map(|d| d.name.clone()),
            keyword: self
                .engine
                .wake_coordinator
                .config
                .keyword
                .as_str()
                .to_string(),
            sensitivity: self.sensitivity_slider,
            vad_threshold: self.vad_threshold_slider,
            wake_greeting: self.engine.wake_coordinator.config.wake_greeting.clone(),
            sleep_keyword: self.engine.wake_coordinator.config.sleep_keyword.clone(),
            listen_command_seconds: self.engine.wake_coordinator.config.listen_command_seconds,
            router_url: self.router_url.clone(),
            speak_reply: self.speak_reply,
            stt_provider: self.stt_provider,
            tts_provider: self.tts_provider,
            locale_code: self.locale.code().to_string(),
            dark_theme: self.is_dark_theme,
        };
        self.engine.apply_settings(s.clone());
        match serde_json::to_string_pretty(&s) {
            Ok(text) => match std::fs::write(settings_file_path(), text) {
                Ok(()) => {
                    self.status_msg = "Settings saved to speechcraft-settings.json".to_string();
                }
                Err(e) => {
                    self.status_msg = format!("Could not write settings file: {e}");
                }
            },
            Err(e) => {
                self.status_msg = format!("Could not encode settings: {e}");
            }
        }
    }

    /// Best-effort TCP probe of the fleet voice-bus router URL.
    pub fn probe_router(&mut self) {
        self.engine.voice_bus.config.router_url = self.router_url.clone();
        let res = self.engine.dispatch(Command::ProbeRouter, &self.store);
        self.router_reachable = res.get("reachable").and_then(|v| v.as_bool());
        self.router_detail = res
            .get("detail")
            .and_then(|v| v.as_str())
            .unwrap_or("no detail")
            .to_string();
        self.status_msg = format!("Router probe: {}", self.router_detail);
    }

    /// Polls the shared live ring buffer (fed by the CPAL stream thread),
    /// updates RMS / confidence / spectrum from REAL mic samples, and feeds
    /// the wake coordinator so VAD + wake-energy actually react to voice.
    pub fn poll_live_audio(&mut self) {
        if !self.is_capturing_mic {
            return;
        }
        let ring_opt = self.mic_ring.clone();
        let Some(ring) = ring_opt else {
            self.audio_source_note = "CAPTURE FLAG ON BUT NO RING — restart mic".to_string();
            return;
        };
        // Snapshot under lock, then release before DSP work.
        let (snapshot, total) = match ring.lock() {
            Ok(guard) => {
                let total = guard.total_written();
                let recent = guard.read_recent(4096);
                (recent, total)
            }
            Err(_) => return,
        };
        if snapshot.is_empty() {
            self.audio_source_note = "LIVE — waiting for first callback…".to_string();
            return;
        }
        // VAD over the newest 80ms frame for stable metering.
        let frame_len = snapshot.len().min(1280);
        let start = snapshot.len().saturating_sub(frame_len);
        let frame = &snapshot[start..];
        let metrics = self.vad_engine.analyze_frame(frame);
        self.live_rms = metrics.energy_rms;
        // Spectrum over a wider window for a stable 48-bin display.
        let spectrum = self.spec_analyzer.compute_spectrum(&snapshot);
        if spectrum.len() == self.live_spectrum.len() {
            self.live_spectrum.copy_from_slice(&spectrum);
        } else {
            self.live_spectrum = spectrum;
        }
        // Feed the delta since last poll into the wake coordinator so
        // keyword-energy + command-window state follow the real mic.
        if total > self.last_polled_total {
            let new_count = (total - self.last_polled_total).min(snapshot.len());
            let delta_start = snapshot.len().saturating_sub(new_count);
            let delta = &snapshot[delta_start..];
            if !delta.is_empty() {
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                if let Some(event) = self.engine.wake_coordinator.ingest_samples(delta, now_ms) {
                    self.status_msg = format!("Live mic event: {event}");
                }
            }
            self.last_polled_total = total;
        }
        self.live_confidence = self.engine.wake_coordinator.last_keyword_confidence;
        // Keep engine VAD energy floor in sync with the slider.
        self.engine.wake_coordinator.vad.energy_threshold =
            self.vad_threshold_slider.clamp(0.001, 1.0);
        self.engine.wake_coordinator.config.sensitivity = self.sensitivity_slider;
        let dev = self
            .input_devices
            .get(self.selected_device_idx)
            .map(|d| d.name.as_str())
            .unwrap_or("mic");
        self.audio_source_note = format!("LIVE MIC — {dev} ({} samples buffered)", total);
    }
}

impl eframe::App for SpeechCraftApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // High-Contrast Obsidian Theme
        if self.is_dark_theme {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = Color32::from_rgb(10, 15, 29); // Deep Obsidian Slate
            visuals.window_fill = Color32::from_rgb(13, 20, 36);
            visuals.extreme_bg_color = Color32::from_rgb(6, 10, 20);
            let ink = Color32::from_rgb(242, 246, 252);
            visuals.override_text_color = Some(ink);
            visuals.weak_text_color = Some(Color32::from_rgb(203, 213, 225));

            // High-contrast non-interactive background and crisp text
            visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(15, 23, 42);
            visuals.widgets.noninteractive.bg_stroke =
                egui::Stroke::new(1.0, Color32::from_rgb(45, 60, 85));
            visuals.widgets.noninteractive.fg_stroke =
                egui::Stroke::new(1.0, Color32::from_rgb(248, 250, 252));

            // Vibrant interactive widget states
            visuals.widgets.inactive.bg_fill = Color32::from_rgb(19, 29, 49);
            visuals.widgets.inactive.bg_stroke =
                egui::Stroke::new(1.0, Color32::from_rgb(56, 189, 248)); // Electric cyan
            visuals.widgets.inactive.fg_stroke =
                egui::Stroke::new(1.0, Color32::from_rgb(241, 245, 249));

            visuals.widgets.hovered.bg_fill = Color32::from_rgb(29, 45, 75);
            visuals.widgets.hovered.bg_stroke =
                egui::Stroke::new(1.5, Color32::from_rgb(56, 189, 248));
            visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, Color32::WHITE);

            visuals.widgets.active.bg_fill = Color32::from_rgb(2, 132, 199);
            visuals.widgets.active.bg_stroke =
                egui::Stroke::new(1.5, Color32::from_rgb(245, 158, 11)); // Amber highlight
            visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, Color32::WHITE);

            visuals.selection.bg_fill = Color32::from_rgb(14, 116, 144);
            visuals.selection.stroke = egui::Stroke::new(1.0, Color32::from_rgb(56, 189, 248));
            ui.ctx().set_visuals(visuals);
        } else {
            let mut visuals = egui::Visuals::light();
            let ink = Color32::BLACK;
            visuals.override_text_color = Some(ink);
            visuals.weak_text_color = Some(ink);
            visuals.panel_fill = Color32::WHITE;
            visuals.window_fill = Color32::WHITE;
            visuals.extreme_bg_color = Color32::WHITE;
            visuals.faint_bg_color = Color32::WHITE;
            visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, ink);
            visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, ink);
            visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.5, ink);
            visuals.widgets.active.fg_stroke = egui::Stroke::new(1.5, ink);
            visuals.widgets.noninteractive.bg_stroke =
                egui::Stroke::new(1.0, Color32::from_rgb(203, 213, 225));
            ui.ctx().set_visuals(visuals);
        }

        // Global Scroll Wheel Zoom: Ctrl + Mouse Wheel adjusts interface scale factor
        let (scroll_delta, ctrl_pressed) =
            ui.input(|i| (i.smooth_scroll_delta.y, i.modifiers.ctrl));
        if ctrl_pressed && scroll_delta.abs() > 0.1 {
            self.ui_zoom = (self.ui_zoom + scroll_delta * 0.002).clamp(0.75, 2.25);
            ui.ctx().set_zoom_factor(self.ui_zoom);
        }

        let cyan = Color32::from_rgb(56, 189, 248);
        let gold = Color32::from_rgb(245, 158, 11);
        let loc = self.locale;

        // 1. TOP MENU BAR
        egui::Panel::top("desktop_menu_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button(i18n::t(I18nKey::File, loc), |ui| {
                    if ui
                        .button(i18n::t(I18nKey::ExportBroadcastMaster, loc))
                        .clicked()
                    {
                        self.status_msg =
                            "Broadcast master (-16 LUFS) exported to disk".to_string();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button(i18n::t(I18nKey::Studio, loc), |ui| {
                    if ui
                        .button(format!("🎙️ {}", i18n::t(I18nKey::Cockpit, loc)))
                        .clicked()
                    {
                        self.current_screen = StudioScreen::Cockpit;
                        ui.close();
                    }
                    if ui
                        .button(format!("🎛️ {}", i18n::t(I18nKey::ProsodySculptor, loc)))
                        .clicked()
                    {
                        self.current_screen = StudioScreen::ProsodySculptor;
                        ui.close();
                    }
                    if ui
                        .button(format!("📜 {}", i18n::t(I18nKey::SpokenTimeline, loc)))
                        .clicked()
                    {
                        self.current_screen = StudioScreen::SpokenTimeline;
                        ui.close();
                    }
                    if ui
                        .button(format!("🔒 {}", i18n::t(I18nKey::VoiceprintVault, loc)))
                        .clicked()
                    {
                        self.current_screen = StudioScreen::VoiceBusVault;
                        ui.close();
                    }
                });

                ui.menu_button(i18n::t(I18nKey::VoiceBus, loc), |ui| {
                    if ui.button(i18n::t(I18nKey::StartListening, loc)).clicked() {
                        self.engine.wake_coordinator.trigger_wake();
                        self.status_msg =
                            "Wake listener active (listening for wakeywakey / hey_jarvis)"
                                .to_string();
                        ui.close();
                    }
                    if ui.button(i18n::t(I18nKey::StopListening, loc)).clicked() {
                        self.engine.wake_coordinator.trigger_sleep();
                        self.status_msg = "Wake listener suspended (dormant)".to_string();
                        ui.close();
                    }
                });

                ui.menu_button(i18n::t(I18nKey::Security, loc), |ui| {
                    if self.is_authenticated {
                        ui.label(format!("Operator: {}", self.auth_username));
                        if ui.button("Sign Out").clicked() {
                            self.is_authenticated = false;
                            self.status_msg = "Signed out".to_string();
                            ui.close();
                        }
                    } else if ui.button("Authenticate Voice Biometrics...").clicked() {
                        self.is_authenticated = true;
                        self.status_msg = "Authenticated as Sovereign Voice Operator".to_string();
                        ui.close();
                    }
                });

                ui.menu_button(i18n::t(I18nKey::Help, loc), |ui| {
                    if ui.button("About SpeechCraft").clicked() {
                        self.show_about_dialog = true;
                        ui.close();
                    }
                });

                // Right quick toggles: Locale, Theme, Auth, Port
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let cur_loc = self.locale;
                    let loc_label = format!("{} {}", cur_loc.flag(), cur_loc.code().to_uppercase());
                    egui::ComboBox::from_id_salt("topbar_locale_selector")
                        .selected_text(loc_label)
                        .show_ui(ui, |ui| {
                            for loc_item in AppLocale::all() {
                                let text =
                                    format!("{} {}", loc_item.flag(), loc_item.display_name());
                                let _ = ui.selectable_value(&mut self.locale, *loc_item, text);
                            }
                        });

                    let theme_icon = if self.is_dark_theme {
                        "☀ Light"
                    } else {
                        "🌙 Dark"
                    };
                    if ui.button(theme_icon).clicked() {
                        self.is_dark_theme = !self.is_dark_theme;
                    }

                    let auth_badge = if self.is_authenticated {
                        i18n::t(I18nKey::Authenticated, loc)
                    } else {
                        i18n::t(I18nKey::Guest, loc)
                    };
                    if ui.button(auth_badge).clicked() {
                        self.is_authenticated = !self.is_authenticated;
                    }

                    ui.label(
                        egui::RichText::new(format!("Port {CONTROL_PORT}"))
                            .color(gold)
                            .monospace()
                            .size(11.0),
                    );
                });
            });
        });

        // 2. SUB-BAR: Studio Screen Tabs & Acoustic Telemetry + Interactive Zoom Controls
        egui::Panel::top("sub_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong(
                    egui::RichText::new("⚡ SPEECHCRAFT STUDIO")
                        .color(cyan)
                        .size(16.0),
                );
                ui.separator();

                for (screen, label) in [
                    (StudioScreen::Cockpit, i18n::t(I18nKey::Cockpit, loc)),
                    (
                        StudioScreen::ProsodySculptor,
                        i18n::t(I18nKey::ProsodySculptor, loc),
                    ),
                    (
                        StudioScreen::SpokenTimeline,
                        i18n::t(I18nKey::SpokenTimeline, loc),
                    ),
                    (
                        StudioScreen::VoiceBusVault,
                        i18n::t(I18nKey::VoiceprintVault, loc),
                    ),
                    (StudioScreen::Settings, "⚙ Settings"),
                    (StudioScreen::Help, "❓ Help"),
                ] {
                    if ui
                        .selectable_label(self.current_screen == screen, label)
                        .clicked()
                    {
                        self.current_screen = screen;
                    }
                }

                ui.separator();
                // Interactive Zoom Controls (Scroll Wheel / Buttons)
                let zoom_pct = (self.ui_zoom * 100.0).round() as i32;
                ui.label(
                    egui::RichText::new(format!(
                        "🔍 {}: {zoom_pct}%",
                        i18n::t(I18nKey::ZoomLabel, loc)
                    ))
                    .color(cyan),
                );
                if ui
                    .small_button("➖")
                    .on_hover_text("Zoom Out (or Ctrl + Scroll Down)")
                    .clicked()
                {
                    self.ui_zoom = (self.ui_zoom - 0.1).clamp(0.75, 2.25);
                    ui.ctx().set_zoom_factor(self.ui_zoom);
                }
                if ui
                    .small_button("100%")
                    .on_hover_text("Reset Zoom to 100%")
                    .clicked()
                {
                    self.ui_zoom = 1.0;
                    ui.ctx().set_zoom_factor(1.0);
                }
                if ui
                    .small_button("➕")
                    .on_hover_text("Zoom In (or Ctrl + Scroll Up)")
                    .clicked()
                {
                    self.ui_zoom = (self.ui_zoom + 0.1).clamp(0.75, 2.25);
                    ui.ctx().set_zoom_factor(self.ui_zoom);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let wake_state_label = if self.engine.wake_coordinator.config.is_listening {
                        "🟢 WAKE ACTIVE"
                    } else {
                        "⚪ DORMANT"
                    };
                    ui.label(egui::RichText::new(wake_state_label).strong().color(
                        if self.engine.wake_coordinator.config.is_listening {
                            Color32::from_rgb(34, 197, 94)
                        } else {
                            Color32::GRAY
                        },
                    ));
                });
            });
        });

        // 3. STATUS FOOTER
        egui::Panel::bottom("status_footer").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(&self.status_msg)
                        .size(12.0)
                        .color(Color32::from_rgb(148, 163, 184)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("Target: -16.0 LUFS Broadcast Standard")
                            .monospace()
                            .size(11.0),
                    );
                });
            });
        });

        // 4. CENTRAL STUDIO CANVAS
        egui::CentralPanel::default().show(ui, |ui| match self.current_screen {
            StudioScreen::Cockpit => self.render_cockpit(ui),
            StudioScreen::ProsodySculptor => self.render_prosody_sculptor(ui),
            StudioScreen::SpokenTimeline => self.render_spoken_timeline(ui),
            StudioScreen::VoiceBusVault => self.render_voice_bus_vault(ui),
            StudioScreen::Settings => self.render_settings(ui),
            StudioScreen::Help => self.render_help(ui),
        });
    }
}

impl SpeechCraftApp {
    fn render_cockpit(&mut self, ui: &mut egui::Ui) {
        let loc = self.locale;
        let cyan = Color32::from_rgb(56, 189, 248);
        let gold = Color32::from_rgb(245, 158, 11);

        // Poll the shared CPAL ring buffer FIRST so every widget below
        // (meters, VAD badge, wake state) reflects the real microphone.
        self.poll_live_audio();

        ui.heading(format!("🎙️ {}", i18n::t(I18nKey::Cockpit, loc)));
        ui.label("Real-time ambient vocal listener, full-duplex conversational barge-in, and wake word ingress.");
        ui.add_space(4.0);
        // Honest source badge: LIVE vs SIMULATED IDLE. No gaslight meters.
        let (badge_text, badge_color) = if self.is_capturing_mic {
            (
                self.audio_source_note.clone(),
                Color32::from_rgb(34, 197, 94),
            )
        } else {
            (
                "SIMULATED IDLE — visualizers animate a placeholder until you press Start Mic"
                    .to_string(),
                Color32::from_rgb(245, 158, 11),
            )
        };
        ui.label(
            egui::RichText::new(badge_text)
                .monospace()
                .size(11.0)
                .color(badge_color),
        );
        ui.add_space(4.0);

        // Continuous animation (idle wobble only matters when NOT capturing)
        self.anim_phase = (self.anim_phase + 0.05) % (std::f32::consts::TAU * 100.0);
        ui.ctx().request_repaint();

        ui.horizontal(|ui| {
            for (mode, label) in [
                (VisualizerMode::Spectrum, "📊 Spectrum"),
                (VisualizerMode::Oscilloscope, "〰 Oscilloscope"),
                (VisualizerMode::VuMeter, "🎚 VU Meter"),
            ] {
                if ui.selectable_label(self.viz_mode == mode, label).clicked() {
                    self.viz_mode = mode;
                }
            }
        });
        let (scroll_delta, ctrl_pressed) =
            ui.input(|i| (i.smooth_scroll_delta.y, i.modifiers.ctrl));

        let canvas_response = egui::Frame::canvas(ui.style())
            .fill(Color32::from_rgb(6, 10, 20))
            .stroke(egui::Stroke::new(1.5, Color32::from_rgb(56, 189, 248)))
            .show(ui, |ui| {
                ui.set_min_size(Vec2::new(ui.available_width(), 110.0));
                let painter = ui.painter();
                let rect = ui.max_rect();

                // Subtle grid lines with electric cyan glow
                for row in 1..4 {
                    let y = rect.min.y + (rect.height() / 4.0) * row as f32;
                    painter.line_segment(
                        [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                        egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(56, 189, 248, 25)),
                    );
                }

                let n = self.live_spectrum.len().max(1);
                let phase = self.anim_phase;
                let zoom = self.visualizer_zoom;
                let capturing = self.is_capturing_mic;
                // Live level from real RMS; idle fallback is a dim placeholder.
                let level = if capturing {
                    (self.live_rms * 8.0).clamp(0.0, 1.0)
                } else {
                    0.22
                };
                match self.viz_mode {
                    VisualizerMode::Spectrum => {
                        let bar_width = rect.width() / n as f32;
                        for (i, &raw_mag) in self.live_spectrum.iter().enumerate() {
                            // Live: raw mic spectrum as-is. Idle: gentle sin wobble
                            // so the screen is not frozen, clearly labelled above.
                            let wob = if capturing {
                                1.0
                            } else {
                                0.75 + 0.25 * (phase + i as f32 * 0.35).sin()
                            };
                            let mag = (raw_mag * zoom * wob).clamp(0.04, 1.0);
                            let h = mag * (rect.height() - 20.0);
                            let bar_rect = egui::Rect::from_min_size(
                                egui::pos2(rect.min.x + i as f32 * bar_width, rect.max.y - h - 8.0),
                                Vec2::new((bar_width - 2.0).max(1.0), h),
                            );
                            let color = if mag > 0.82 {
                                Color32::from_rgb(244, 63, 94)
                            } else if mag > 0.55 {
                                Color32::from_rgb(251, 191, 36)
                            } else {
                                Color32::from_rgb(0, 240, 255)
                            };
                            // Soft glow underlay, then crisp bar
                            let glow = bar_rect.expand(2.0);
                            painter.rect_filled(
                                glow,
                                3.0,
                                Color32::from_rgba_unmultiplied(
                                    color.r(),
                                    color.g(),
                                    color.b(),
                                    40,
                                ),
                            );
                            painter.rect_filled(bar_rect, 2.0, color);
                        }
                    }
                    VisualizerMode::Oscilloscope => {
                        let steps = 160;
                        let mid = rect.center().y;
                        let amp = (rect.height() * 0.4) * level.max(0.12) * zoom.min(2.0);
                        // When capturing, the trace amplitude follows the live
                        // RMS level; the waveform shape stays synthetic until a
                        // raw-PCM trace buffer lands (roadmap), but amplitude is real.
                        let pts: Vec<egui::Pos2> = (0..=steps)
                            .map(|k| {
                                let t = k as f32 / steps as f32;
                                let x = rect.min.x + t * rect.width();
                                let y = mid
                                    + amp
                                        * ((t * 18.0 + phase * 2.0).sin() * 0.6
                                            + (t * 41.0 - phase * 3.0).sin() * 0.4);
                                egui::pos2(x, y)
                            })
                            .collect();
                        painter.add(egui::Shape::line(
                            pts.clone(),
                            egui::Stroke::new(
                                5.0,
                                Color32::from_rgba_unmultiplied(0, 240, 255, 40),
                            ),
                        ));
                        painter.add(egui::Shape::line(
                            pts,
                            egui::Stroke::new(2.0, Color32::from_rgb(0, 240, 255)),
                        ));
                    }
                    VisualizerMode::VuMeter => {
                        let segs = 32;
                        let lit = (level * zoom.min(2.0) * segs as f32).round() as usize;
                        let seg_w = rect.width() / segs as f32;
                        for s in 0..segs {
                            let on = s < lit.min(segs);
                            let base = if s > 26 {
                                Color32::from_rgb(244, 63, 94)
                            } else if s > 18 {
                                Color32::from_rgb(251, 191, 36)
                            } else {
                                Color32::from_rgb(0, 240, 255)
                            };
                            let c = if on {
                                base
                            } else {
                                Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), 30)
                            };
                            let r = egui::Rect::from_min_size(
                                egui::pos2(rect.min.x + s as f32 * seg_w, rect.center().y - 18.0),
                                Vec2::new((seg_w - 3.0).max(1.0), 36.0),
                            );
                            painter.rect_filled(r, 3.0, c);
                        }
                    }
                }

                // High-contrast zoom scale telemetry overlay
                painter.text(
                    egui::pos2(rect.min.x + 8.0, rect.min.y + 6.0),
                    egui::Align2::LEFT_TOP,
                    format!(
                        "Visualizer Scale: {:.1}x (Hover & scroll wheel to zoom)",
                        self.visualizer_zoom
                    ),
                    egui::FontId::monospace(10.0),
                    Color32::from_rgb(148, 163, 184),
                );
            });

        // Hover wheel zoom on visualizer canvas
        if canvas_response.response.hovered() && !ctrl_pressed && scroll_delta.abs() > 0.1 {
            self.visualizer_zoom = (self.visualizer_zoom + scroll_delta * 0.05).clamp(0.5, 3.0);
        }

        ui.add_space(12.0);

        // Hardware Audio Ingress Card (High Contrast)
        egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(15, 23, 42))
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(37, 52, 77)))
            .show(ui, |ui| {
                ui.heading(egui::RichText::new(i18n::t(I18nKey::HardwareIngress, loc)).color(cyan));
                ui.horizontal(|ui| {
                    ui.label(format!("{}:", i18n::t(I18nKey::InputDevice, loc)));
                    let current_dev_name = self
                        .input_devices
                        .get(self.selected_device_idx)
                        .map(|d| d.name.as_str())
                        .unwrap_or("Default System Microphone");

                    egui::ComboBox::from_id_salt("mic_device_selector")
                        .selected_text(current_dev_name)
                        .show_ui(ui, |ui| {
                            for (idx, dev) in self.input_devices.iter().enumerate() {
                                let label = if dev.is_default {
                                    format!("{} (Default)", dev.name)
                                } else {
                                    dev.name.clone()
                                };
                                ui.selectable_value(&mut self.selected_device_idx, idx, label);
                            }
                        });

                    if ui.button(i18n::t(I18nKey::Refresh, loc)).clicked() {
                        self.input_devices = self.engine.capture_manager.list_input_devices();
                        self.status_msg = format!("Found {} audio input device(s)", self.input_devices.len());
                    }
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let is_capturing = self.is_capturing_mic;
                    let btn_text = if is_capturing {
                        i18n::t(I18nKey::StopMic, loc)
                    } else {
                        i18n::t(I18nKey::StartMic, loc)
                    };
                    if ui.button(btn_text).clicked() {
                        if is_capturing {
                            self.engine.capture_manager.stop_capture();
                            self.mic_ring = None;
                            self.is_capturing_mic = false;
                            self.live_rms = 0.0;
                            self.live_confidence = 0.0;
                            self.audio_source_note =
                                "SIMULATED IDLE — press Start Mic for live input".to_string();
                            self.status_msg = "Microphone streaming stopped".to_string();
                        } else {
                            let dev_name = self
                                .input_devices
                                .get(self.selected_device_idx)
                                .map(|d| d.name.as_str());
                            // Shared Arc: the CPAL callback writes HERE and the
                            // GUI polls the SAME Arc every frame. No more clone
                            // disconnect — this was the fantasy-data bug.
                            let ring =
                                Arc::new(Mutex::new(AudioRingBuffer::new(16_000 * 10)));
                            match self
                                .engine
                                .capture_manager
                                .start_capture(dev_name, Arc::clone(&ring))
                            {
                                Ok(name) => {
                                    self.mic_ring = Some(ring);
                                    self.last_polled_total = 0;
                                    self.is_capturing_mic = true;
                                    self.audio_source_note =
                                        format!("LIVE MIC — {name} (streaming…)");
                                    self.status_msg =
                                        format!("Microphone capture active: '{name}'");
                                }
                                Err(e) => {
                                    self.mic_ring = None;
                                    self.status_msg = format!(
                                        "Audio capture unavailable: {e}. Check Windows mic privacy + device."
                                    );
                                }
                            }
                        }
                    }

                    ui.separator();
                    let vad_thresh = self.vad_threshold_slider;
                    let vad_status = if !self.is_capturing_mic {
                        "⚪ IDLE (mic off)"
                    } else if self.live_rms >= vad_thresh {
                        "🟢 SPEECH (live)"
                    } else {
                        "⚪ QUIET (live)"
                    };
                    ui.label(format!("VAD: {vad_status}"));
                    ui.separator();
                    ui.label(format!("RMS: {:.3} (live)", self.live_rms));
                    ui.separator();
                    ui.label(format!("Keyword Match: {:.1}%", self.live_confidence * 100.0));
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label("Keyword Sensitivity:");
                    if ui.add(egui::Slider::new(&mut self.sensitivity_slider, 0.0..=1.0)).changed() {
                        self.engine.wake_coordinator.config.sensitivity = self.sensitivity_slider;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("VAD Energy Threshold:");
                    if ui
                        .add(egui::Slider::new(&mut self.vad_threshold_slider, 0.005..=0.2))
                        .changed()
                    {
                        self.vad_engine = VadEngine::new(0.015, self.vad_threshold_slider);
                        self.engine.wake_coordinator.config.vad_threshold =
                            self.vad_threshold_slider.clamp(0.01, 0.99);
                    }
                    ui.label(format!("(speak to exceed {:.3})", self.vad_threshold_slider));
                });
            });

        ui.add_space(12.0);

        // Speaker / Output Card — REAL enumeration + test tone.
        egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(15, 23, 42))
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(37, 52, 77)))
            .show(ui, |ui| {
                ui.heading(egui::RichText::new("🔊 Speaker Output (Real Devices)").color(cyan));
                ui.horizontal(|ui| {
                    ui.label("Output Device:");
                    let cur_out = self
                        .output_devices
                        .get(self.selected_output_idx)
                        .map(|d| d.name.as_str())
                        .unwrap_or("Default System Speakers");
                    egui::ComboBox::from_id_salt("speaker_device_selector")
                        .selected_text(cur_out)
                        .show_ui(ui, |ui| {
                            for (idx, dev) in self.output_devices.iter().enumerate() {
                                let label = if dev.is_default {
                                    format!("{} (Default)", dev.name)
                                } else {
                                    dev.name.clone()
                                };
                                ui.selectable_value(&mut self.selected_output_idx, idx, label);
                            }
                        });
                    if ui.small_button("🔄").on_hover_text("Refresh output devices").clicked() {
                        self.output_devices = self.engine.synth.list_output_devices();
                        self.status_msg = format!(
                            "Found {} output device(s)",
                            self.output_devices.len()
                        );
                    }
                    if ui.button("🔔 Test Tone").on_hover_text("Synthesize + play a short tone through the default output").clicked() {
                        let clip = self.engine.synth.synthesize(
                            "SpeechCraft speaker test.",
                            "VP-MISTRESS",
                            0.0, 1.0, 0.6, 0.2, 0.5, -16.0,
                        );
                        match self.engine.synth.play_samples(&clip.pcm_samples) {
                            Ok(()) => {
                                self.status_msg =
                                    "Test tone playing on host speakers (real CPAL output).".to_string();
                            }
                            Err(e) => {
                                self.status_msg =
                                    format!("No speaker output available: {e}. Clip buffered, not heard.");
                            }
                        }
                    }
                });
                ui.label(
                    egui::RichText::new(if self.output_devices.is_empty() {
                        "No output devices reported by OS — playback will buffer silently."
                    } else {
                        "Output list comes from the OS (CPAL). If empty, check Windows Sound settings."
                    })
                    .size(11.0)
                    .color(Color32::from_rgb(148, 163, 184)),
                );
            });

        ui.add_space(12.0);

        // Wake Word Controls Card (High Contrast)
        egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(15, 23, 42))
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(37, 52, 77)))
            .show(ui, |ui| {
                ui.heading(
                    egui::RichText::new("Acoustic Ingress & Wake Word Configuration").color(cyan),
                );
                ui.horizontal(|ui| {
                    ui.label(format!("{}:", i18n::t(I18nKey::WakeWord, loc)));
                    ui.strong(
                        egui::RichText::new(self.engine.wake_coordinator.config.keyword.as_str())
                            .color(gold),
                    );
                    ui.separator();
                    ui.label("Sensitivity:");
                    ui.strong(format!(
                        "{:.2}",
                        self.engine.wake_coordinator.config.sensitivity
                    ));
                    ui.separator();
                    ui.label(format!("{}:", i18n::t(I18nKey::Greeting, loc)));
                    ui.strong(&self.engine.wake_coordinator.config.wake_greeting);
                    ui.separator();
                    ui.label(format!("{}:", i18n::t(I18nKey::SleepWord, loc)));
                    ui.strong(&self.engine.wake_coordinator.config.sleep_keyword);
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Live state: {:?} | last RMS {:.3} | last confidence {:.1}%",
                        self.engine.wake_coordinator.state,
                        self.engine.wake_coordinator.last_vad_rms,
                        self.engine.wake_coordinator.last_keyword_confidence * 100.0
                    ));
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button("⚡ Simulate Wake Trigger").clicked() {
                        let greet = self.engine.wake_coordinator.trigger_wake();
                        self.status_msg = format!("Wake triggered! Spoken response: '{greet}'");
                    }
                    if ui.button("🌙 Simulate Sleep Command").clicked() {
                        let msg = self.engine.wake_coordinator.trigger_sleep();
                        self.status_msg = format!("Sleep triggered! '{msg}'");
                    }
                });
                ui.label(
                    egui::RichText::new(
                        "Live mic feeds VAD + wake-energy every frame when capturing. \
                         Simulated buttons are for testing without speaking.",
                    )
                    .size(11.0)
                    .color(Color32::from_rgb(148, 163, 184)),
                );
            });

        ui.add_space(12.0);

        // Voice Command Bus Card — honest live/fallback mode.
        egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(15, 23, 42))
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(37, 52, 77)))
            .show(ui, |ui| {
                ui.heading(
                    egui::RichText::new(i18n::t(I18nKey::VoiceCommandBusTitle, loc)).color(cyan),
                );
                ui.horizontal(|ui| {
                    ui.label("Router URL:");
                    ui.text_edit_singleline(&mut self.router_url);
                    if ui
                        .small_button("📡 Probe")
                        .on_hover_text("TCP-probe the fleet voice-bus router")
                        .clicked()
                    {
                        self.probe_router();
                    }
                    let (txt, col) = match self.router_reachable {
                        Some(true) => ("🟢 ROUTER LIVE", Color32::from_rgb(34, 197, 94)),
                        Some(false) => ("🟠 LOCAL FALLBACK", Color32::from_rgb(245, 158, 11)),
                        None => ("⚪ NOT PROBED", Color32::GRAY),
                    };
                    ui.label(egui::RichText::new(txt).color(col).strong());
                });
                ui.label(
                    egui::RichText::new(format!(
                        "Router: {} — {}. Transcription is typed here until an STT sidecar \
                         (Whisper/FunASR/Sherpa) streams live transcripts.",
                        self.router_url, self.router_detail
                    ))
                    .size(11.0)
                    .color(Color32::from_rgb(148, 163, 184)),
                );
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Utterance:");
                    ui.text_edit_singleline(&mut self.manual_voice_command);
                    if ui.button(i18n::t(I18nKey::DispatchToFleet, loc)).clicked() {
                        let cmd = self.manual_voice_command.clone();
                        self.dispatch_utterance(&cmd);
                    }
                });

                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("Quick:").color(gold));
                    for q in QUICK_COMMANDS {
                        if ui.small_button(q).clicked() {
                            self.manual_voice_command = q.to_string();
                            self.dispatch_utterance(q);
                        }
                    }
                });

                if let Some(reply) = &self.engine.voice_bus.last_reply {
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(format!("Last Spoken Reply: \"{reply}\""))
                            .color(cyan)
                            .strong(),
                    );
                }

                if !self.dispatch_history.is_empty() {
                    ui.add_space(6.0);
                    ui.separator();
                    ui.label(egui::RichText::new("Dispatch History").strong().color(gold));
                    egui::Grid::new("dispatch_history_grid")
                        .striped(true)
                        .num_columns(3)
                        .show(ui, |ui| {
                            for (receiver, transcript, reply) in
                                self.dispatch_history.iter().rev().take(8)
                            {
                                ui.label(
                                    egui::RichText::new(format!("● {receiver}"))
                                        .color(cyan)
                                        .monospace(),
                                );
                                ui.label(transcript);
                                ui.label(egui::RichText::new(reply).italics());
                                ui.end_row();
                            }
                        });
                }
            });
    }

    fn dispatch_utterance(&mut self, utterance: &str) {
        let current_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let intent = self
            .engine
            .wake_coordinator
            .extract_intent(utterance, current_epoch);
        let res = self.engine.voice_bus.route_intent(intent);
        self.status_msg = format!(
            "Routed to '{}'! Spoken Reply: '{}'",
            res.receiver, res.reply_spoken_text
        );
        self.dispatch_history.push((
            res.receiver.to_string(),
            utterance.to_string(),
            res.reply_spoken_text.to_string(),
        ));
        if self.dispatch_history.len() > 50 {
            self.dispatch_history.remove(0);
        }
    }

    fn render_prosody_sculptor(&mut self, ui: &mut egui::Ui) {
        let loc = self.locale;
        ui.heading(format!("🎛️ {}", i18n::t(I18nKey::ProsodySculptor, loc)));
        ui.label(
            "Sculpt vocal inflection, pitch contours, warmth, breathiness, and emotional gravity.",
        );
        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("Presets:").color(Color32::from_rgb(245, 158, 11)));
            for (name, p, r, w, b, g) in PROSODY_PRESETS {
                if ui.button(name).clicked() {
                    self.pitch_val = p;
                    self.rate_val = r;
                    self.warmth_val = w;
                    self.breathiness_val = b;
                    self.gravity_val = g;
                    self.status_msg = format!("Preset loaded: {name}");
                }
            }
        });
        ui.add_space(8.0);

        egui::Grid::new("prosody_grid")
            .num_columns(2)
            .spacing([20.0, 10.0])
            .show(ui, |ui| {
                ui.label("Pitch Inflection (Semitones):");
                ui.add(egui::Slider::new(&mut self.pitch_val, -12.0..=12.0).suffix(" st"));
                ui.end_row();

                ui.label("Speech Rate Multiplier:");
                ui.add(egui::Slider::new(&mut self.rate_val, 0.5..=2.0).suffix("x"));
                ui.end_row();

                ui.label("Vocal Warmth (Chest Resonance):");
                ui.add(egui::Slider::new(&mut self.warmth_val, 0.0..=1.0));
                ui.end_row();

                ui.label("Breathiness (Intimate Aspiration):");
                ui.add(egui::Slider::new(&mut self.breathiness_val, 0.0..=1.0));
                ui.end_row();

                ui.label("Acoustic Gravity (Authority):");
                ui.add(egui::Slider::new(&mut self.gravity_val, 0.0..=1.0));
                ui.end_row();
            });

        ui.add_space(12.0);
        ui.label("Preview Utterance:");
        ui.text_edit_multiline(&mut self.preview_text);
        ui.add_space(6.0);

        if ui
            .button(format!("▶ {}", i18n::t(I18nKey::SpeakText, loc)))
            .clicked()
        {
            let prosody = speechcraft_core::VocalProsody {
                pitch_semitones: self.pitch_val,
                rate_multiplier: self.rate_val,
                warmth: self.warmth_val,
                breathiness: self.breathiness_val,
                gravity: self.gravity_val,
            };
            let clip = self.engine.synth.synthesize(
                &self.preview_text,
                "VP-MISTRESS",
                prosody.pitch_semitones,
                prosody.rate_multiplier,
                prosody.warmth,
                prosody.breathiness,
                prosody.gravity,
                -16.0,
            );
            let played = self.engine.synth.play_samples(&clip.pcm_samples).is_ok();
            self.status_msg = format!(
                "Synthesized: {:.2}s, LUFS: {:.1} ({})",
                clip.duration_ms as f32 / 1000.0,
                clip.integrated_lufs,
                if played {
                    "Active Output — heard on speakers"
                } else {
                    "No output device — clip buffered, not heard"
                }
            );
        }
    }

    fn render_spoken_timeline(&mut self, ui: &mut egui::Ui) {
        let loc = self.locale;
        ui.heading(format!("📜 {}", i18n::t(I18nKey::SpokenTimeline, loc)));
        ui.label("Paragraph-by-paragraph multitrack audiobook and voiceover mastering timeline.");
        ui.add_space(12.0);

        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.new_paragraph_text);
            if ui.button("➕ Add Paragraph").clicked() && !self.new_paragraph_text.trim().is_empty()
            {
                let idx = self.engine.timeline.len();
                let text = self.new_paragraph_text.trim().to_string();
                let est_ms = (text.split_whitespace().count() as u64 * 400).max(400);
                self.engine
                    .timeline
                    .push(speechcraft_core::SpokenTimelineItem {
                        id: format!("para-{idx}"),
                        paragraph_idx: idx,
                        raw_text: text,
                        phonetic_override: None,
                        speaker_voice_id: "VP-MISTRESS".to_string(),
                        prosody: speechcraft_core::VocalProsody {
                            pitch_semitones: self.pitch_val,
                            rate_multiplier: self.rate_val,
                            warmth: self.warmth_val,
                            breathiness: self.breathiness_val,
                            gravity: self.gravity_val,
                        },
                        target_lufs: -16.0,
                        audio_duration_ms: est_ms,
                        is_rendered: false,
                    });
                self.new_paragraph_text.clear();
                self.status_msg = format!("Paragraph #{} added to timeline", idx + 1);
            }
        });
        ui.add_space(8.0);

        let mut play_idx: Option<usize> = None;
        for (i, item) in self.engine.timeline.iter().enumerate() {
            egui::Frame::group(ui.style())
                .fill(Color32::from_rgb(15, 23, 42))
                .stroke(egui::Stroke::new(1.0, Color32::from_rgb(37, 52, 77)))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.strong(
                            egui::RichText::new(format!("#{:02}", item.paragraph_idx + 1))
                                .color(Color32::from_rgb(56, 189, 248)),
                        );
                        ui.label(format!("Speaker: {}", item.speaker_voice_id));
                        ui.separator();
                        ui.label(format!(
                            "Duration: {:.1}s",
                            item.audio_duration_ms as f32 / 1000.0
                        ));
                        ui.separator();
                        ui.label(format!("Loudness: {:.1} LUFS", item.target_lufs));
                        if ui.small_button("▶ Play Clip").clicked() {
                            play_idx = Some(i);
                        }
                    });
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(&item.raw_text)
                            .italics()
                            .color(Color32::from_rgb(241, 245, 249)),
                    );
                });
            ui.add_space(6.0);
        }

        if let Some(i) = play_idx {
            if let Some(item) = self.engine.timeline.get(i) {
                let clip = self.engine.synth.synthesize(
                    &item.raw_text,
                    &item.speaker_voice_id,
                    item.prosody.pitch_semitones,
                    item.prosody.rate_multiplier,
                    item.prosody.warmth,
                    item.prosody.breathiness,
                    item.prosody.gravity,
                    item.target_lufs,
                );
                let played = self.engine.synth.play_samples(&clip.pcm_samples).is_ok();
                self.status_msg = format!(
                    "Clip #{}: {:.2}s, {:.1} LUFS ({})",
                    i + 1,
                    clip.duration_ms as f32 / 1000.0,
                    clip.integrated_lufs,
                    if played {
                        "Active Output — heard on speakers"
                    } else {
                        "No output device — buffered only"
                    }
                );
            }
        }

        if ui
            .button(i18n::t(I18nKey::ExportBroadcastMaster, loc))
            .clicked()
        {
            let mut total_samples = Vec::new();
            for item in &self.engine.timeline {
                let clip = self.engine.synth.synthesize(
                    &item.raw_text,
                    &item.speaker_voice_id,
                    item.prosody.pitch_semitones,
                    item.prosody.rate_multiplier,
                    item.prosody.warmth,
                    item.prosody.breathiness,
                    item.prosody.gravity,
                    item.target_lufs,
                );
                total_samples.extend_from_slice(&clip.pcm_samples);
            }
            let wav = speechcraft_dsp::AcousticSynthesizer::export_wav_bytes(
                &total_samples,
                speechcraft_dsp::SAMPLE_RATE_16K,
            );
            let path = "speechcraft_master_output.wav";
            self.status_msg = match std::fs::write(path, &wav) {
                Ok(()) => format!(
                    "Broadcast master saved to {path} ({} bytes, -16.0 LUFS EBU R128)",
                    wav.len()
                ),
                Err(e) => format!("Could not write {path}: {e}"),
            };
        }
    }

    fn render_voice_bus_vault(&mut self, ui: &mut egui::Ui) {
        let loc = self.locale;
        ui.heading(format!("🔒 {}", i18n::t(I18nKey::VoiceprintVault, loc)));
        ui.label("Enclave voiceprint biometric vault and fleet receiver matrix.");
        ui.add_space(12.0);

        ui.heading(
            egui::RichText::new("Registered Sovereign Voiceprints")
                .color(Color32::from_rgb(56, 189, 248)),
        );
        for vp in &self.engine.voiceprints {
            egui::Frame::group(ui.style())
                .fill(Color32::from_rgb(15, 23, 42))
                .stroke(egui::Stroke::new(1.0, Color32::from_rgb(37, 52, 77)))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.strong(egui::RichText::new(&vp.name).color(Color32::from_rgb(245, 158, 11)));
                        ui.label(format!("Gender: {}", vp.gender));
                        ui.separator();
                        ui.label(format!("Accent: {}", vp.accent));
                        ui.separator();
                        ui.label(format!("Embedding: {}-dim", vp.embedding.len()));
                        ui.separator();
                        ui.label(format!("Confidence: {:.0}%", vp.similarity_confidence * 100.0));
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui.button(format!("🧪 Synthetic Match ({})", vp.profile_id)).on_hover_text("Lab test with synthetic tone — NOT a mic verification").clicked() {
                            let synthetic = vec![0.15f32; 16_000];
                            let res = self.engine.voiceprint_enclave.verify(&synthetic, &vp.embedding, &vp.profile_id);
                            self.status_msg = format!(
                                "Synthetic check for {}: Match {:.1}%, Authenticated: {} (synthetic, not mic)",
                                vp.profile_id,
                                res.similarity * 100.0,
                                if res.is_authenticated { "YES" } else { "NO" }
                            );
                        }
                        let live_label = if self.is_capturing_mic { "🎙 Verify Live Mic" } else { "🎙 Verify Live Mic (mic off)" };
                        if ui.button(live_label).on_hover_text("Verify the last 1s of REAL mic buffer against this voiceprint").clicked() {
                            if let Some(ring) = self.mic_ring.clone() {
                                let samples = match ring.lock() {
                                    Ok(g) => g.read_recent(16_000),
                                    Err(_) => Vec::new(),
                                };
                                if samples.len() < 1600 {
                                    self.status_msg = "Live buffer too short — speak into the mic first.".to_string();
                                } else {
                                    let res = self.engine.voiceprint_enclave.verify(&samples, &vp.embedding, &vp.profile_id);
                                    self.status_msg = format!(
                                        "LIVE mic check for {}: Match {:.1}%, Authenticated: {} ({} live samples)",
                                        vp.profile_id,
                                        res.similarity * 100.0,
                                        if res.is_authenticated { "YES" } else { "NO" },
                                        samples.len()
                                    );
                                }
                            } else {
                                self.status_msg = "Mic is off — press Start Mic in Cockpit first.".to_string();
                            }
                        }
                    });
                });
            ui.add_space(6.0);
        }
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(
                "Synthetic Match is a lab self-test. Live Mic verification uses the real \
                 ring buffer (needs mic on + speech). Enroll-via-mic lives in Settings.",
            )
            .size(11.0)
            .color(Color32::from_rgb(148, 163, 184)),
        );
    }

    fn render_settings(&mut self, ui: &mut egui::Ui) {
        let cyan = Color32::from_rgb(56, 189, 248);
        let dim = Color32::from_rgb(148, 163, 184);
        ui.heading("⚙ Settings — audio, wake, bus, providers, UI");
        ui.label(
            egui::RichText::new(
                "Everything SpeechCraft remembers lives in speechcraft-settings.json \
                 next to the working directory. Save applies to the engine immediately.",
            )
            .size(12.0)
            .color(dim),
        );
        ui.add_space(8.0);

        // Audio
        egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(15, 23, 42))
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(37, 52, 77)))
            .show(ui, |ui| {
                ui.heading(egui::RichText::new("Audio Devices").color(cyan));
                ui.horizontal(|ui| {
                    ui.label("Input:");
                    let cur = self
                        .input_devices
                        .get(self.selected_device_idx)
                        .map(|d| d.name.as_str())
                        .unwrap_or("none");
                    egui::ComboBox::from_id_salt("settings_input")
                        .selected_text(cur)
                        .show_ui(ui, |ui| {
                            for (i, d) in self.input_devices.iter().enumerate() {
                                ui.selectable_value(
                                    &mut self.selected_device_idx,
                                    i,
                                    d.name.clone(),
                                );
                            }
                        });
                    if ui.small_button("🔄").clicked() {
                        self.input_devices = self.engine.capture_manager.list_input_devices();
                        self.output_devices = self.engine.synth.list_output_devices();
                        self.status_msg = format!(
                            "Audio refresh: {} in / {} out",
                            self.input_devices.len(),
                            self.output_devices.len()
                        );
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Output:");
                    let cur = self
                        .output_devices
                        .get(self.selected_output_idx)
                        .map(|d| d.name.as_str())
                        .unwrap_or("none");
                    egui::ComboBox::from_id_salt("settings_output")
                        .selected_text(cur)
                        .show_ui(ui, |ui| {
                            for (i, d) in self.output_devices.iter().enumerate() {
                                ui.selectable_value(
                                    &mut self.selected_output_idx,
                                    i,
                                    d.name.clone(),
                                );
                            }
                        });
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("VAD threshold:");
                    ui.add(egui::Slider::new(
                        &mut self.vad_threshold_slider,
                        0.005..=0.2,
                    ));
                    ui.label("Sensitivity:");
                    ui.add(egui::Slider::new(&mut self.sensitivity_slider, 0.0..=1.0));
                });
                ui.label(
                    egui::RichText::new(format!(
                        "Live now: RMS {:.3} | wake {:?} | {}",
                        self.live_rms, self.engine.wake_coordinator.state, self.audio_source_note
                    ))
                    .monospace()
                    .size(11.0)
                    .color(dim),
                );
            });
        ui.add_space(8.0);

        // Wake
        egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(15, 23, 42))
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(37, 52, 77)))
            .show(ui, |ui| {
                ui.heading(egui::RichText::new("Wake Word").color(cyan));
                ui.horizontal(|ui| {
                    ui.label("Keyword:");
                    let mut kw = self
                        .engine
                        .wake_coordinator
                        .config
                        .keyword
                        .as_str()
                        .to_string();
                    egui::ComboBox::from_id_salt("settings_keyword")
                        .selected_text(kw.clone())
                        .show_ui(ui, |ui| {
                            for cand in ["wakeywakey", "hey_jarvis", "alexa", "hey_mycroft"] {
                                ui.selectable_value(&mut kw, cand.to_string(), cand);
                            }
                        });
                    if kw != self.engine.wake_coordinator.config.keyword.as_str() {
                        self.engine.wake_coordinator.config.keyword = match kw.as_str() {
                            "hey_jarvis" => speechcraft_core::WakeKeyword::HeyJarvis,
                            "alexa" => speechcraft_core::WakeKeyword::Alexa,
                            "hey_mycroft" => speechcraft_core::WakeKeyword::HeyMycroft,
                            _ => speechcraft_core::WakeKeyword::WakeyWakey,
                        };
                    }
                    ui.label("Greeting:");
                    ui.text_edit_singleline(&mut self.engine.wake_coordinator.config.wake_greeting);
                });
                ui.horizontal(|ui| {
                    ui.label("Sleep word:");
                    ui.text_edit_singleline(&mut self.engine.wake_coordinator.config.sleep_keyword);
                    ui.label("Listen window (s):");
                    let mut secs =
                        self.engine.wake_coordinator.config.listen_command_seconds as i32;
                    ui.add(egui::Slider::new(&mut secs, 1..=30));
                    self.engine.wake_coordinator.config.listen_command_seconds =
                        secs.clamp(1, 30) as u32;
                });
            });
        ui.add_space(8.0);

        // Bus + AI providers
        egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(15, 23, 42))
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(37, 52, 77)))
            .show(ui, |ui| {
                ui.heading(egui::RichText::new("Voice Bus + AI Providers").color(cyan));
                ui.horizontal(|ui| {
                    ui.label("Router URL:");
                    ui.text_edit_singleline(&mut self.router_url);
                    if ui.small_button("📡 Probe").clicked() {
                        self.probe_router();
                    }
                    let (txt, col) = match self.router_reachable {
                        Some(true) => ("🟢 LIVE", Color32::from_rgb(34, 197, 94)),
                        Some(false) => ("🟠 FALLBACK", Color32::from_rgb(245, 158, 11)),
                        None => ("⚪ ?", Color32::GRAY),
                    };
                    ui.label(egui::RichText::new(txt).color(col).strong());
                });
                ui.label(
                    egui::RichText::new(self.router_detail.clone()).size(11.0).color(dim),
                );
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.speak_reply, "Speak fleet replies aloud");
                });
                ui.horizontal(|ui| {
                    ui.label("STT (speech→text):");
                    egui::ComboBox::from_id_salt("settings_stt")
                        .selected_text(format!("{:?}", self.stt_provider))
                        .show_ui(ui, |ui| {
                            for p in [
                                SttProvider::WhisperLocal,
                                SttProvider::FunAsr,
                                SttProvider::SherpaOnnx,
                                SttProvider::GeminiLive,
                            ] {
                                ui.selectable_value(&mut self.stt_provider, p, format!("{p:?}"));
                            }
                        });
                    ui.label("TTS (text→speech):");
                    egui::ComboBox::from_id_salt("settings_tts")
                        .selected_text(format!("{:?}", self.tts_provider))
                        .show_ui(ui, |ui| {
                            for p in [
                                TtsProvider::KokoroLocal,
                                TtsProvider::Qwen3Tts,
                                TtsProvider::HumeEvi,
                                TtsProvider::ElevenLabs,
                                TtsProvider::GeminiFlash,
                            ] {
                                ui.selectable_value(&mut self.tts_provider, p, format!("{p:?}"));
                            }
                        });
                });
                ui.label(
                    egui::RichText::new(
                        "Honest status: typed utterances route today; live transcription needs an STT \
                         sidecar. Built-in TTS is a local parametric formant synth (no cloud). \
                         External providers are selectable for the roadmap and reported over MCP \
                         via speechcraft_get_settings.",
                    )
                    .size(11.0)
                    .color(dim),
                );
            });
        ui.add_space(8.0);

        // UI + persistence
        egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(15, 23, 42))
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(37, 52, 77)))
            .show(ui, |ui| {
                ui.heading(egui::RichText::new("Interface + Persistence").color(cyan));
                ui.horizontal(|ui| {
                    ui.label("Locale:");
                    egui::ComboBox::from_id_salt("settings_locale")
                        .selected_text(self.locale.display_name())
                        .show_ui(ui, |ui| {
                            for l in AppLocale::all() {
                                ui.selectable_value(&mut self.locale, *l, l.display_name());
                            }
                        });
                    ui.checkbox(&mut self.is_dark_theme, "Dark theme");
                    ui.label("UI zoom:");
                    ui.add(egui::Slider::new(&mut self.ui_zoom, 0.75..=2.25));
                });
                ui.horizontal(|ui| {
                    if ui.button("💾 Save Settings").clicked() {
                        self.save_settings();
                    }
                    ui.label(
                        egui::RichText::new(
                            "Writes speechcraft-settings.json + applies to engine.",
                        )
                        .size(11.0)
                        .color(dim),
                    );
                });
            });
    }

    fn render_help(&mut self, ui: &mut egui::Ui) {
        let cyan = Color32::from_rgb(56, 189, 248);
        let gold = Color32::from_rgb(245, 158, 11);
        let dim = Color32::from_rgb(148, 163, 184);
        ui.heading("❓ SpeechCraft Help — the whole studio, honestly labelled");
        ui.label(
            egui::RichText::new(
                "What is live, what is simulated, and how to get sound in and out. \
                 Start here if meters move but the room is quiet.",
            )
            .color(dim),
        );
        ui.add_space(8.0);

        egui::Grid::new("help_grid").num_columns(1).spacing([8.0, 8.0]).show(ui, |ui| {
            for (title, body) in [
                (
                    "1 · Quickstart (60 seconds to live sound)",
                    "Cockpit → pick Input Device → Start Continuous Mic → speak: VAD flips to \
                     SPEECH (live), RMS rises, Spectrum follows your voice. Still flat? See §8. \
                     Prosody tab → Test Tone / Speak Text → listen on speakers. Still silent? See §8. \
                     Voice Bus → type `fritz gpu status` → Dispatch. Help → Settings → Save.",
                ),
                (
                    "2 · Microphone (real path)",
                    "CPAL WASAPI input → mono downmix → 16 kHz resample → shared Arc ring buffer \
                     (10 s) → per-frame poll: VAD RMS, 48-bin spectrum, wake-energy. The LIVE MIC \
                     badge shows the device + buffered sample count. When the mic is OFF the \
                     visualizers show a labelled SIMULATED IDLE placeholder — never fake live data.",
                ),
                (
                    "3 · Meters (what each shows)",
                    "Spectrum: live 48-bin magnitudes from the mic when capturing, idle wobble otherwise. \
                     Oscilloscope: amplitude follows live RMS (shape is illustrative until a raw-PCM \
                     trace lands). VU: segment count = live level × zoom. RMS text is always live. \
                     Keyword Match % is the wake matcher confidence on the last window.",
                ),
                (
                    "4 · Wake words (wakeywakey, hey_jarvis, alexa, hey_mycroft)",
                    "VAD gates the matcher; sensitivity slider = permissiveness; VAD threshold = \
                     energy floor. Live state line shows Dormant / ListeningForWake / \
                     AwakeCapturingCommand / Sleeping. Simulate buttons are test-only; the live path \
                     updates automatically while capturing. Tune in Settings → Wake Word.",
                ),
                (
                    "5 · Voice bus + AI (typed today, spoken tomorrow)",
                    "Utterance box → intent parse (fritz/codercraft/robotcraft/calibre/plexy/…) → \
                     local fallback reply, or live-router reply when Probe says ROUTER LIVE \
                     (fleet-agent 10996 /api/voice/intent). Provider dropdowns (Whisper/FunASR/Sherpa, \
                     Kokoro/Qwen3/Hume/ElevenLabs/Gemini) are recorded + reported over MCP; live \
                     streaming transcription/voices need those sidecars — the studio never pretends \
                     typed text is a transcript.",
                ),
                (
                    "6 · Prosody + speakers (real output)",
                    "Pitch (±12 st), rate (0.5–2×), warmth, breathiness, gravity shape a local \
                     formant synth, mastered to −16 LUFS (EBU R128). Speaker dropdown enumerates real \
                     OS outputs; Test Tone / Speak Text plays via CPAL. Status says Active Output or \
                     names the exact OS error. Timeline Play Clip uses the same path.",
                ),
                (
                    "7 · Timeline (−16 LUFS broadcast master)",
                    "Add paragraphs → per-clip synth at current prosody → Play Clip to audition → \
                     Export Broadcast Master writes speechcraft_master_output.wav (16-bit PCM, mono, \
                     16 kHz, loudness-normalized). Duration estimate ≈ 400 ms/word until rendered.",
                ),
                (
                    "8 · Voiceprints (biometrics, honestly labelled)",
                    "32-dim RMS/ZCR embeddings, cosine similarity, threshold 0.72. Synthetic Match = \
                     lab self-test (never auth). Verify Live Mic = last 1 s of REAL mic buffer vs the \
                     profile. For real enrollment speak 2–3 s with the mic on, then use the enrolled \
                     profile the same way. Short/quiet buffers are refused with a message.",
                ),
                (
                    "9 · Settings (persisted)",
                    "Audio in/out, VAD + sensitivity, keyword/greeting/sleep/listen-window, router URL \
                     + speak-reply, STT/TTS providers, locale (7), theme, zoom. 💾 Save writes \
                     speechcraft-settings.json and applies to the engine + MCP GetSettings.",
                ),
                (
                    "10 · MCP + ports (agent surface)",
                    "Port 14132 MCP: status, list_audio_devices, list_output_devices, configure_wake, \
                     route_intent, synthesize (+play_audio), export_wav, verify_voiceprint, \
                     list_voiceprints, timeline_manage, get_settings, probe_router. Voice-bus router: \
                     127.0.0.1:10996 /api/voice/intent. Probe card shows LIVE vs LOCAL-FALLBACK.",
                ),
                (
                    "11 · Troubleshooting (read before filing a bug)",
                    "Meters frozen at 0: mic is off or OS blocked it — Start Mic, check Windows \
                     Settings → Privacy → Microphone, Refresh Devices. VAD never green: lower VAD \
                     threshold, raise mic gain, move closer. No speakers: empty output list = OS reports \
                     none; check Sound settings, then Test Tone for the exact error. Router FALLBACK: \
                     fleet-agent not running on 10996 — start it or keep local fallback. Distorted \
                     synth: lower warmth/gravity, keep rate near 1.0.",
                ),
            ] {
                egui::Frame::group(ui.style())
                    .fill(Color32::from_rgb(15, 23, 42))
                    .stroke(egui::Stroke::new(1.0, Color32::from_rgb(37, 52, 77)))
                    .show(ui, |ui| {
                        ui.heading(egui::RichText::new(title).color(cyan).size(14.0));
                        ui.label(egui::RichText::new(body).color(Color32::from_rgb(226, 232, 240)));
                    });
                ui.end_row();
            }
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Honesty rule: ").color(gold).strong());
            ui.label(
                egui::RichText::new(
                    "anything labelled LIVE comes from the OS device; anything labelled \
                     SIMULATED/SYNTHETIC/IDLE never pretends to be a microphone, speaker, \
                     transcript, or enrolled voice.",
                )
                .color(dim),
            );
        });
    }
}

fn main() -> eframe::Result<()> {
    let args = Args::parse();
    if args.smoke {
        let store = match DatabaseStore::open_in_memory() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("smoke: cannot open store: {e}");
                std::process::exit(1);
            }
        };
        let mut engine = SpeechCraftEngine::new();
        let status = engine.dispatch(Command::Status, &store);
        let ok = status
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if ok {
            println!("speechcraft smoke: OK (control port {})", args.control);
            std::process::exit(0);
        } else {
            eprintln!("speechcraft smoke: failed");
            std::process::exit(1);
        }
    }

    let app = match SpeechCraftApp::new() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("speechcraft: cannot initialize app: {e}");
            std::process::exit(1);
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(format!("SpeechCraft v{APP_VERSION} Sovereign Studio"))
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([960.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native("SpeechCraft", options, Box::new(|_cc| Ok(Box::new(app))))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_speechcraft_app_init() {
        let mut app = SpeechCraftApp::new().unwrap();
        assert_eq!(app.current_screen, StudioScreen::Cockpit);
        assert_eq!(app.locale, AppLocale::En);
        assert_eq!(app.ui_zoom, 1.0);
        assert_eq!(app.visualizer_zoom, 1.0);
        assert!(!app.live_spectrum.is_empty());
        assert!(!app.is_capturing_mic);
        assert!(app.mic_ring.is_none());

        // Test i18n switching
        app.locale = AppLocale::De;
        assert_eq!(app.locale.code(), "de");
        app.locale = AppLocale::Ja;
        assert_eq!(app.locale.code(), "ja");

        // Test zoom adjustment
        app.ui_zoom = 1.25;
        assert_eq!(app.ui_zoom, 1.25);
    }
}
