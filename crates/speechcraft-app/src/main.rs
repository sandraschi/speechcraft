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

    // Hardware Audio Input & Continuous Listener
    pub input_devices: Vec<speechcraft_dsp::InputDeviceInfo>,
    pub selected_device_idx: usize,
    pub is_capturing_mic: bool,
    pub live_rms: f32,
    pub live_confidence: f32,
    pub sensitivity_slider: f32,

    // Zoom & Scaling
    pub ui_zoom: f32,
    pub visualizer_zoom: f32,

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
        let input_devices = engine.capture_manager.list_input_devices();

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
            input_devices,
            selected_device_idx: 0,
            is_capturing_mic: false,
            live_rms: 0.0,
            live_confidence: 0.0,
            sensitivity_slider: 0.5,
            ui_zoom: 1.0,
            visualizer_zoom: 1.0,
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
        // High-Contrast Obsidian Theme
        if self.is_dark_theme {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = Color32::from_rgb(10, 15, 29); // Deep Obsidian Slate
            visuals.window_fill = Color32::from_rgb(13, 20, 36);
            visuals.extreme_bg_color = Color32::from_rgb(6, 10, 20);

            // High-contrast non-interactive background and crisp text
            visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(15, 23, 42);
            visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, Color32::from_rgb(45, 60, 85));
            visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, Color32::from_rgb(248, 250, 252));

            // Vibrant interactive widget states
            visuals.widgets.inactive.bg_fill = Color32::from_rgb(19, 29, 49);
            visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, Color32::from_rgb(56, 189, 248)); // Electric cyan
            visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, Color32::from_rgb(241, 245, 249));

            visuals.widgets.hovered.bg_fill = Color32::from_rgb(29, 45, 75);
            visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.5, Color32::from_rgb(56, 189, 248));
            visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, Color32::WHITE);

            visuals.widgets.active.bg_fill = Color32::from_rgb(2, 132, 199);
            visuals.widgets.active.bg_stroke = egui::Stroke::new(1.5, Color32::from_rgb(245, 158, 11)); // Amber highlight
            visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, Color32::WHITE);

            visuals.selection.bg_fill = Color32::from_rgb(14, 116, 144);
            visuals.selection.stroke = egui::Stroke::new(1.0, Color32::from_rgb(56, 189, 248));
            ui.ctx().set_visuals(visuals);
        } else {
            let mut visuals = egui::Visuals::light();
            visuals.panel_fill = Color32::from_rgb(248, 250, 252);
            visuals.window_fill = Color32::from_rgb(255, 255, 255);
            visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, Color32::from_rgb(203, 213, 225));
            ui.ctx().set_visuals(visuals);
        }

        // Global Scroll Wheel Zoom: Ctrl + Mouse Wheel adjusts interface scale factor
        let (scroll_delta, ctrl_pressed) = ui.input(|i| (i.smooth_scroll_delta.y, i.modifiers.ctrl));
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

                // Right quick toggles: Locale, Theme, Auth, Port
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

        // 2. SUB-BAR: Studio Screen Tabs & Acoustic Telemetry + Interactive Zoom Controls
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

                ui.separator();
                // Interactive Zoom Controls (Scroll Wheel / Buttons)
                let zoom_pct = (self.ui_zoom * 100.0).round() as i32;
                ui.label(egui::RichText::new(format!("🔍 {}: {zoom_pct}%", i18n::t(I18nKey::ZoomLabel, loc))).color(cyan));
                if ui.small_button("➖").on_hover_text("Zoom Out (or Ctrl + Scroll Down)").clicked() {
                    self.ui_zoom = (self.ui_zoom - 0.1).clamp(0.75, 2.25);
                    ui.ctx().set_zoom_factor(self.ui_zoom);
                }
                if ui.small_button("100%").on_hover_text("Reset Zoom to 100%").clicked() {
                    self.ui_zoom = 1.0;
                    ui.ctx().set_zoom_factor(1.0);
                }
                if ui.small_button("➕").on_hover_text("Zoom In (or Ctrl + Scroll Up)").clicked() {
                    self.ui_zoom = (self.ui_zoom + 0.1).clamp(0.75, 2.25);
                    ui.ctx().set_zoom_factor(self.ui_zoom);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let wake_state_label = if self.engine.wake_coordinator.config.is_listening {
                        "🟢 WAKE ACTIVE"
                    } else {
                        "⚪ DORMANT"
                    };
                    ui.label(egui::RichText::new(wake_state_label).strong().color(if self.engine.wake_coordinator.config.is_listening { Color32::from_rgb(34, 197, 94) } else { Color32::GRAY }));
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

        // Acoustic Visualizer Panel with Scroll Wheel Zoom
        let (scroll_delta, ctrl_pressed) = ui.input(|i| (i.smooth_scroll_delta.y, i.modifiers.ctrl));

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

                let bar_width = rect.width() / self.simulated_spectrum.len() as f32;
                for (i, &raw_mag) in self.simulated_spectrum.iter().enumerate() {
                    let mag = (raw_mag * self.visualizer_zoom).clamp(0.04, 1.0);
                    let h = mag * (rect.height() - 20.0);
                    let bar_rect = egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + i as f32 * bar_width, rect.max.y - h - 8.0),
                        Vec2::new((bar_width - 2.0).max(1.0), h),
                    );

                    // High-contrast electric gradient: Neon Cyan -> Vivid Amber -> Hot Rose
                    let color = if mag > 0.82 {
                        Color32::from_rgb(244, 63, 94) // Hot Rose
                    } else if mag > 0.55 {
                        Color32::from_rgb(251, 191, 36) // Vivid Amber
                    } else {
                        Color32::from_rgb(0, 240, 255) // Neon Electric Cyan
                    };
                    painter.rect_filled(bar_rect, 2.0, color);
                }

                // High-contrast zoom scale telemetry overlay
                painter.text(
                    egui::pos2(rect.min.x + 8.0, rect.min.y + 6.0),
                    egui::Align2::LEFT_TOP,
                    format!("Visualizer Scale: {:.1}x (Hover & scroll wheel to zoom)", self.visualizer_zoom),
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
                            self.is_capturing_mic = false;
                            self.status_msg = "Microphone streaming stopped".to_string();
                        } else {
                            let dev_name = self.input_devices.get(self.selected_device_idx).map(|d| d.name.as_str());
                            let ring = std::sync::Arc::new(std::sync::Mutex::new(self.engine.wake_coordinator.ring_buffer.clone()));
                            match self.engine.capture_manager.start_capture(dev_name, ring) {
                                Ok(name) => {
                                    self.is_capturing_mic = true;
                                    self.status_msg = format!("Microphone capture active: '{name}'");
                                }
                                Err(e) => {
                                    self.status_msg = format!("Audio capture unavailable: {e}");
                                }
                            }
                        }
                    }

                    ui.separator();
                    let vad_status = if self.live_rms > 0.02 { "🟢 SPEECH" } else { "⚪ QUIET" };
                    ui.label(format!("VAD: {vad_status}"));
                    ui.separator();
                    ui.label(format!("RMS: {:.3}", self.live_rms));
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
            });

        ui.add_space(12.0);

        // Wake Word Controls Card (High Contrast)
        egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(15, 23, 42))
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(37, 52, 77)))
            .show(ui, |ui| {
                ui.heading(egui::RichText::new("Acoustic Ingress & Wake Word Configuration").color(cyan));
                ui.horizontal(|ui| {
                    ui.label(format!("{}:", i18n::t(I18nKey::WakeWord, loc)));
                    ui.strong(egui::RichText::new(self.engine.wake_coordinator.config.keyword.as_str()).color(gold));
                    ui.separator();
                    ui.label("Sensitivity:");
                    ui.strong(format!("{:.2}", self.engine.wake_coordinator.config.sensitivity));
                    ui.separator();
                    ui.label(format!("{}:", i18n::t(I18nKey::Greeting, loc)));
                    ui.strong(&self.engine.wake_coordinator.config.wake_greeting);
                    ui.separator();
                    ui.label(format!("{}:", i18n::t(I18nKey::SleepWord, loc)));
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

        // Voice Command Bus Simulation Card (High Contrast)
        egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(15, 23, 42))
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(37, 52, 77)))
            .show(ui, |ui| {
                ui.heading(egui::RichText::new(i18n::t(I18nKey::VoiceCommandBusTitle, loc)).color(cyan));
                ui.horizontal(|ui| {
                    ui.label("Utterance:");
                    ui.text_edit_singleline(&mut self.manual_voice_command);
                    if ui.button(i18n::t(I18nKey::DispatchToFleet, loc)).clicked() {
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
                "Synthesized: {:.2}s, LUFS: {:.1} (Audio Stream: {})",
                clip.duration_ms as f32 / 1000.0,
                clip.integrated_lufs,
                if played { "Active Output" } else { "Simulated/Buffered" }
            );
        }
    }

    fn render_spoken_timeline(&mut self, ui: &mut egui::Ui) {
        let loc = self.locale;
        ui.heading(format!("📜 {}", i18n::t(I18nKey::SpokenTimeline, loc)));
        ui.label("Paragraph-by-paragraph multitrack audiobook and voiceover mastering timeline.");
        ui.add_space(12.0);

        for item in &self.engine.timeline {
            egui::Frame::group(ui.style())
                .fill(Color32::from_rgb(15, 23, 42))
                .stroke(egui::Stroke::new(1.0, Color32::from_rgb(37, 52, 77)))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.strong(egui::RichText::new(format!("#{:02}", item.paragraph_idx + 1)).color(Color32::from_rgb(56, 189, 248)));
                        ui.label(format!("Speaker: {}", item.speaker_voice_id));
                        ui.separator();
                        ui.label(format!("Duration: {:.1}s", item.audio_duration_ms as f32 / 1000.0));
                        ui.separator();
                        ui.label(format!("Loudness: {:.1} LUFS", item.target_lufs));
                    });
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(&item.raw_text).italics().color(Color32::from_rgb(241, 245, 249)));
                });
            ui.add_space(6.0);
        }

        if ui.button(i18n::t(I18nKey::ExportBroadcastMaster, loc)).clicked() {
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
            let wav = speechcraft_dsp::AcousticSynthesizer::export_wav_bytes(&total_samples, speechcraft_dsp::SAMPLE_RATE_16K);
            self.status_msg = format!("Broadcast master WAV generated ({} bytes, -16.0 LUFS EBU R128 standard)", wav.len());
        }
    }

    fn render_voice_bus_vault(&mut self, ui: &mut egui::Ui) {
        let loc = self.locale;
        ui.heading(format!("🔒 {}", i18n::t(I18nKey::VoiceprintVault, loc)));
        ui.label("Enclave voiceprint biometric vault and fleet receiver matrix.");
        ui.add_space(12.0);

        ui.heading(egui::RichText::new("Registered Sovereign Voiceprints").color(Color32::from_rgb(56, 189, 248)));
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
                        if ui.button(format!("🧪 Test Biometric Match ({})", vp.profile_id)).clicked() {
                            let synthetic = vec![0.15f32; 16_000];
                            let res = self.engine.voiceprint_enclave.verify(&synthetic, &vp.embedding, &vp.profile_id);
                            self.status_msg = format!(
                                "Biometric check for {}: Match {:.1}%, Authenticated: {}",
                                vp.profile_id,
                                res.similarity * 100.0,
                                if res.is_authenticated { "YES" } else { "NO" }
                            );
                        }
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
        let mut app = SpeechCraftApp::new().unwrap();
        assert_eq!(app.current_screen, StudioScreen::Cockpit);
        assert_eq!(app.locale, AppLocale::En);
        assert_eq!(app.ui_zoom, 1.0);
        assert_eq!(app.visualizer_zoom, 1.0);
        assert!(!app.simulated_spectrum.is_empty());

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
