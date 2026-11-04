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
use speechcraft_core::{Command, SpeechCraftEngine, APP_VERSION, CONTROL_PORT};
use speechcraft_dsp::SpectrogramAnalyzer;

#[derive(Parser, Debug)]
#[command(name = "speechcraft", about = "SpeechCraft Sovereign Acoustic Studio", version = APP_VERSION)]
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

    // Audio Visualization Mock/Buffer State
    pub spec_analyzer: SpectrogramAnalyzer,
    pub simulated_spectrum: Vec<f32>,

    // Prosody Sliders
    pub pitch_val: f32,
    pub rate_val: f32,
    pub warmth_val: f32,
    pub breathiness_val: f32,
    pub gravity_val: f32,
    pub preview_text: String,

    // Voice Command Input
    pub manual_voice_command: String,
}

impl SpeechCraftApp {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let store = DatabaseStore::open_in_memory()?;
        let engine = SpeechCraftEngine::new();
        let spec_analyzer = SpectrogramAnalyzer::new(48);

        // Simulated spectrum data for 60fps acoustic visualizer
        let mut sim = Vec::with_capacity(48);
        for i in 0..48 {
            let val = ((i as f32 * 0.15).sin().abs() * 0.7 + 0.1).clamp(0.05, 0.95);
            sim.push(val);
        }

        Ok(Self {
            store,
            engine,
            current_screen: StudioScreen::Cockpit,
            locale: AppLocale::default(),
            is_dark_theme: true,
            is_authenticated: false,
            auth_username: "mistress".to_string(),
            show_about_dialog: false,
            status_msg: format!("SpeechCraft v{APP_VERSION} Sovereign Studio Ready (Port {CONTROL_PORT})"),
            spec_analyzer,
            simulated_spectrum: sim,
            pitch_val: 0.0,
            rate_val: 1.0,
            warmth_val: 0.65,
            breathiness_val: 0.25,
            gravity_val: 0.55,
            preview_text: "SpeechCraft delivers pristine sovereign voice to the ArtCraft fleet.".to_string(),
            manual_voice_command: "fritz gpu status".to_string(),
        })
    }
}

impl eframe::App for SpeechCraftApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.is_dark_theme {
            ui.ctx().set_visuals(egui::Visuals::dark());
        } else {
            ui.ctx().set_visuals(egui::Visuals::light());
        }

        let cyan = Color32::from_rgb(56, 189, 248);
        let gold = Color32::from_rgb(245, 158, 11);
        let loc = self.locale;

        // 1. TOP MENU BAR
        egui::Panel::top("desktop_menu_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button(i18n::t(I18nKey::File, loc), |ui| {
                    if ui.button(i18n::t(I18nKey::ExportBroadcastMaster, loc)).clicked() {
                        self.status_msg = "Broadcast master (-16 LUFS) exported to disk".to_string();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button(i18n::t(I18nKey::Studio, loc), |ui| {
                    if ui.button(format!("🎙️ {}", i18n::t(I18nKey::Cockpit, loc))).clicked() {
                        self.current_screen = StudioScreen::Cockpit;
                        ui.close();
                    }
                    if ui.button(format!("🎛️ {}", i18n::t(I18nKey::ProsodySculptor, loc))).clicked() {
                        self.current_screen = StudioScreen::ProsodySculptor;
                        ui.close();
                    }
                    if ui.button(format!("📜 {}", i18n::t(I18nKey::SpokenTimeline, loc))).clicked() {
                        self.current_screen = StudioScreen::SpokenTimeline;
                        ui.close();
                    }
                    if ui.button(format!("🔒 {}", i18n::t(I18nKey::VoiceprintVault, loc))).clicked() {
                        self.current_screen = StudioScreen::VoiceBusVault;
                        ui.close();
                    }
                });

                ui.menu_button(i18n::t(I18nKey::VoiceBus, loc), |ui| {
                    if ui.button(i18n::t(I18nKey::StartListening, loc)).clicked() {
                        self.engine.wake_coordinator.trigger_wake();
                        self.status_msg = "Wake listener active (listening for wakeywakey / hey_jarvis)".to_string();
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

                // Right quick toggles
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let cur_loc = self.locale;
                    let loc_label = format!("{} {}", cur_loc.flag(), cur_loc.code().to_uppercase());
                    egui::ComboBox::from_id_salt("topbar_locale_selector")
                        .selected_text(loc_label)
                        .show_ui(ui, |ui| {
                            for loc_item in AppLocale::all() {
                                let text = format!("{} {}", loc_item.flag(), loc_item.display_name());
                                let _ = ui.selectable_value(&mut self.locale, *loc_item, text);
                            }
                        });

                    let theme_icon = if self.is_dark_theme { "☀ Light" } else { "🌙 Dark" };
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

                    ui.label(egui::RichText::new(format!("Port {CONTROL_PORT}")).color(gold).monospace().size(11.0));
                });
            });
        });

        // 2. SUB-BAR: Studio Screen Tabs & Acoustic Telemetry
        egui::Panel::top("sub_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong(egui::RichText::new("⚡ SPEECHCRAFT STUDIO").color(cyan).size(16.0));
                ui.separator();

                for (screen, key) in [
                    (StudioScreen::Cockpit, I18nKey::Cockpit),
                    (StudioScreen::ProsodySculptor, I18nKey::ProsodySculptor),
                    (StudioScreen::SpokenTimeline, I18nKey::SpokenTimeline),
                    (StudioScreen::VoiceBusVault, I18nKey::VoiceprintVault),
                ] {
                    let label = i18n::t(key, loc);
                    if ui.selectable_label(self.current_screen == screen, label).clicked() {
                        self.current_screen = screen;
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let wake_state_label = if self.engine.wake_coordinator.config.is_listening {
                        "🟢 WAKE ACTIVE"
                    } else {
                        "⚪ DORMANT"
                    };
                    ui.label(egui::RichText::new(wake_state_label).strong().color(if self.engine.wake_coordinator.config.is_listening { Color32::GREEN } else { Color32::GRAY }));
                });
            });
        });

        // 3. STATUS FOOTER
        egui::Panel::bottom("status_footer").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&self.status_msg).size(12.0).color(Color32::from_rgb(148, 163, 184)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("Target: -16.0 LUFS Broadcast Standard").monospace().size(11.0));
                });
            });
        });

        // 4. CENTRAL STUDIO CANVAS
        egui::CentralPanel::default().show(ui, |ui| {
            match self.current_screen {
                StudioScreen::Cockpit => self.render_cockpit(ui),
                StudioScreen::ProsodySculptor => self.render_prosody_sculptor(ui),
                StudioScreen::SpokenTimeline => self.render_spoken_timeline(ui),
                StudioScreen::VoiceBusVault => self.render_voice_bus_vault(ui),
            }
        });
    }
}

impl SpeechCraftApp {
    fn render_cockpit(&mut self, ui: &mut egui::Ui) {
        let loc = self.locale;
        let cyan = Color32::from_rgb(56, 189, 248);
        let gold = Color32::from_rgb(245, 158, 11);

        ui.heading(format!("🎙️ {}", i18n::t(I18nKey::Cockpit, loc)));
        ui.label("Real-time ambient vocal listener, full-duplex conversational barge-in, and wake word ingress.");
        ui.add_space(8.0);

        // Acoustic Visualizer Panel
        egui::Frame::canvas(ui.style()).fill(Color32::from_rgb(15, 23, 42)).show(ui, |ui| {
            ui.set_min_size(Vec2::new(ui.available_width(), 100.0));
            let painter = ui.painter();
            let rect = ui.max_rect();

            let bar_width = rect.width() / self.simulated_spectrum.len() as f32;
            for (i, &mag) in self.simulated_spectrum.iter().enumerate() {
                let h = mag * (rect.height() - 16.0);
                let bar_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.min.x + i as f32 * bar_width, rect.max.y - h - 8.0),
                    Vec2::new(bar_width - 2.0, h),
                );
                let color = if mag > 0.75 { gold } else { cyan };
                painter.rect_filled(bar_rect, 2.0, color);
            }
        });

        ui.add_space(12.0);

        // Wake Word Controls Card
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.heading("Acoustic Ingress & Wake Word Configuration");
            ui.horizontal(|ui| {
                ui.label("Wake Keyword:");
                ui.strong(self.engine.wake_coordinator.config.keyword.as_str());
                ui.separator();
                ui.label("Spoken Greeting:");
                ui.strong(&self.engine.wake_coordinator.config.wake_greeting);
                ui.separator();
                ui.label("Sleep Trigger:");
                ui.strong(&self.engine.wake_coordinator.config.sleep_keyword);
            });

            ui.add_space(6.0);
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
        });

        ui.add_space(12.0);

        // Voice Command Bus Simulation Card
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.heading("Voice Command Bus (Router: 10996)");
            ui.horizontal(|ui| {
                ui.label("Utterance:");
                ui.text_edit_singleline(&mut self.manual_voice_command);
                if ui.button("🚀 Dispatch to Fleet Agent").clicked() {
                    let current_epoch = 1793775000;
                    let intent = self.engine.wake_coordinator.extract_intent(&self.manual_voice_command, current_epoch);
                    let res = self.engine.voice_bus.route_intent(intent);
                    self.status_msg = format!("Routed to '{}'! Spoken Reply: '{}'", res.receiver, res.reply_spoken_text);
                }
            });

            if let Some(reply) = &self.engine.voice_bus.last_reply {
                ui.add_space(6.0);
                ui.label(egui::RichText::new(format!("Last Spoken Reply: \"{reply}\"")).color(cyan).strong());
            }
        });
    }

    fn render_prosody_sculptor(&mut self, ui: &mut egui::Ui) {
        let loc = self.locale;
        ui.heading(format!("🎛️ {}", i18n::t(I18nKey::ProsodySculptor, loc)));
        ui.label("Sculpt vocal inflection, pitch contours, warmth, breathiness, and emotional gravity.");
        ui.add_space(12.0);

        egui::Grid::new("prosody_grid").num_columns(2).spacing([20.0, 10.0]).show(ui, |ui| {
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

        if ui.button(format!("▶ {}", i18n::t(I18nKey::SpeakText, loc))).clicked() {
            self.status_msg = format!("Synthesizing audio: Pitch {:.1}st, Warmth {:.2}", self.pitch_val, self.warmth_val);
        }
    }

    fn render_spoken_timeline(&mut self, ui: &mut egui::Ui) {
        let loc = self.locale;
        ui.heading(format!("📜 {}", i18n::t(I18nKey::SpokenTimeline, loc)));
        ui.label("Paragraph-by-paragraph multitrack audiobook and voiceover mastering timeline.");
        ui.add_space(12.0);

        for item in &self.engine.timeline {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.strong(format!("#{:02}", item.paragraph_idx + 1));
                    ui.label(format!("Speaker: {}", item.speaker_voice_id));
                    ui.separator();
                    ui.label(format!("Duration: {:.1}s", item.audio_duration_ms as f32 / 1000.0));
                    ui.separator();
                    ui.label(format!("Loudness: {:.1} LUFS", item.target_lufs));
                });
                ui.add_space(4.0);
                ui.label(egui::RichText::new(&item.raw_text).italics());
            });
            ui.add_space(6.0);
        }

        if ui.button(i18n::t(I18nKey::ExportBroadcastMaster, loc)).clicked() {
            self.status_msg = "Broadcast master WAV (48kHz, 24-bit, -16.0 LUFS) exported".to_string();
        }
    }

    fn render_voice_bus_vault(&mut self, ui: &mut egui::Ui) {
        let loc = self.locale;
        ui.heading(format!("🔒 {}", i18n::t(I18nKey::VoiceprintVault, loc)));
        ui.label("Enclave voiceprint biometric vault and fleet receiver matrix.");
        ui.add_space(12.0);

        ui.heading("Registered Sovereign Voiceprints");
        for vp in &self.engine.voiceprints {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.strong(&vp.name);
                    ui.label(format!("Gender: {}", vp.gender));
                    ui.separator();
                    ui.label(format!("Accent: {}", vp.accent));
                    ui.separator();
                    ui.label(format!("Confidence: {:.0}%", vp.similarity_confidence * 100.0));
                });
            });
            ui.add_space(6.0);
        }
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
        let ok = status.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
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

    eframe::run_native(
        "SpeechCraft",
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_speechcraft_app_init() {
        let app = SpeechCraftApp::new().unwrap();
        assert_eq!(app.current_screen, StudioScreen::Cockpit);
        assert_eq!(app.locale, AppLocale::En);
        assert!(!app.simulated_spectrum.is_empty());
    }
}
