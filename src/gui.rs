//! The GUI component handles rendering the Kiwi mascot using `egui`.
//! It is responsible for displaying the playful parrot and updating its visual state
//! (e.g., listening, thinking, speaking) seamlessly as an overlay on the desktop.

use eframe::egui;
use tokio::sync::mpsc;

/// Represents the current visual state of the Kiwi mascot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MascotState {
    /// Mascot is hidden or idle.
    Idle,
    /// Mascot is actively listening to the microphone.
    Listening,
    /// Mascot is processing an intent or searching.
    Thinking,
    /// Mascot is outputting text via TTS.
    Speaking,
    /// Mascot is in onboarding state, prompting user to record wakeword.
    Onboarding { recorded: usize, is_recording: bool },
}

/// Trait defining the core behavior of the Kiwi GUI.
pub trait MascotRenderer {
    /// Initializes the GUI window and starts the rendering loop.
    fn run(&mut self) -> Result<(), String>;

    /// Updates the mascot's state to trigger different animations or UI cues.
    fn set_state(&mut self, state: MascotState);
}

/// The main application struct for the Kiwi GUI.
pub struct KiwiGui {
    pub state: MascotState,
    pub rx: mpsc::Receiver<MascotState>,

    // Cached textures for each state
    idle_texture: Option<egui::TextureHandle>,
    listening_texture: Option<egui::TextureHandle>,
    thinking_texture: Option<egui::TextureHandle>,
    speaking_texture: Option<egui::TextureHandle>,

    position_set: bool,

    // Onboarding UI state
    pub templates_recorded: usize,
    pub is_recording: bool,
    pub tx_gui: Option<mpsc::Sender<GuiEvent>>,
}

#[derive(Debug, Clone)]
pub enum GuiEvent {
    RecordSample,
    DoneOnboarding,
    PlaySample(usize),
    DeleteSample(usize),
}

impl KiwiGui {
    pub fn new(rx: mpsc::Receiver<MascotState>, tx_gui: mpsc::Sender<GuiEvent>) -> Self {
        Self {
            state: MascotState::Idle,
            rx,
            idle_texture: None,
            listening_texture: None,
            thinking_texture: None,
            speaking_texture: None,
            position_set: false,
            templates_recorded: 0,
            is_recording: false,
            tx_gui: Some(tx_gui),
        }
    }

    fn load_texture(ctx: &egui::Context, name: &str, path: &str) -> Option<egui::TextureHandle> {
        let image = image::open(path).ok()?;
        let size = [image.width() as _, image.height() as _];
        let image_buffer = image.to_rgba8();
        let pixels = image_buffer.as_flat_samples();
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
        Some(ctx.load_texture(name, color_image, Default::default()))
    }
}

impl eframe::App for KiwiGui {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Poll for state updates from the channel
        while let Ok(new_state) = self.rx.try_recv() {
            self.state = new_state;
        }

        // Initialize textures on first frame
        if self.idle_texture.is_none() {
            self.idle_texture = Self::load_texture(ctx, "idle", "assets/idle.webp");
            self.listening_texture = Self::load_texture(ctx, "listening", "assets/listening.webp");
            self.thinking_texture = Self::load_texture(ctx, "thinking", "assets/thinking.webp");
            self.speaking_texture = Self::load_texture(ctx, "speaking", "assets/speaking.webp");
        }

        // Position window in the bottom-right corner of the monitor once
        if !self.position_set {
            #[allow(clippy::collapsible_if)]
            if let Some(monitor_size) = ctx.input(|i| i.viewport().monitor_size) {
                let window_size = egui::vec2(320.0, 320.0);
                let padding = 20.0;

                let target_pos = egui::pos2(
                    monitor_size.x - window_size.x - padding,
                    monitor_size.y - window_size.y - padding,
                );

                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(target_pos));
                self.position_set = true;
            }
        }

        // Create transparent central panel
        let panel_frame = egui::Frame {
            fill: egui::Color32::TRANSPARENT,
            inner_margin: egui::Margin::same(0),
            ..Default::default()
        };

        #[allow(deprecated)]
        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
                self.ui(ui, frame);
            });

        // Request repaint to ensure smooth transitions if any state changed
        ctx.request_repaint();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let texture = match self.state {
            MascotState::Idle | MascotState::Onboarding { .. } => &self.idle_texture,
            MascotState::Listening => &self.listening_texture,
            MascotState::Thinking => &self.thinking_texture,
            MascotState::Speaking => &self.speaking_texture,
        };

        if let Some(tex) = texture {
            ui.add(egui::Image::new(tex).shrink_to_fit());
        } else {
            // Fallback text if images fail to load
            ui.heading("Kiwi 🦜");
            ui.label(format!("State: {:?}", self.state));
        }

        if let MascotState::Onboarding {
            recorded,
            is_recording,
        } = self.state
        {
            egui::Window::new("Welcome to Kiwi!")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label("Kiwi needs to learn your wake word.");
                    ui.label(format!("Samples recorded: {}/3", recorded));

                    for i in 0..recorded {
                        ui.horizontal(|ui| {
                            ui.label(format!("Sample {}", i + 1));
                            #[allow(clippy::collapsible_if)]
                            if ui.button("▶ Play").clicked() {
                                if let Some(tx) = &self.tx_gui {
                                    let _ = tx.try_send(GuiEvent::PlaySample(i));
                                }
                            }
                            #[allow(clippy::collapsible_if)]
                            if ui.button("🗑 Delete").clicked() {
                                if let Some(tx) = &self.tx_gui {
                                    let _ = tx.try_send(GuiEvent::DeleteSample(i));
                                }
                            }
                        });
                    }

                    if is_recording {
                        ui.label("Recording... Please speak your wake word.");
                    } else {
                        #[allow(clippy::collapsible_if)]
                        if ui.button("Record Sample").clicked() {
                            if let Some(tx) = &self.tx_gui {
                                // Optimistically update state so we don't send multiple events
                                self.state = MascotState::Onboarding {
                                    recorded,
                                    is_recording: true,
                                };
                                let _ = tx.try_send(GuiEvent::RecordSample);
                            }
                        }
                        #[allow(clippy::collapsible_if)]
                        if recorded >= 3 {
                            #[allow(clippy::collapsible_if)]
                            if ui.button("Done").clicked() {
                                if let Some(tx) = &self.tx_gui {
                                    let _ = tx.try_send(GuiEvent::DoneOnboarding);
                                }
                            }
                        }
                    }
                });
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // Completely transparent background
        [0.0, 0.0, 0.0, 0.0]
    }
}

impl MascotRenderer for KiwiGui {
    fn run(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn set_state(&mut self, state: MascotState) {
        self.state = state;
    }
}
