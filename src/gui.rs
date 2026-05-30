/// The GUI component handles rendering the Kiwi mascot using `egui`.
/// It is responsible for displaying the playful parrot and updating its visual state
/// (e.g., listening, thinking, speaking) seamlessly as an overlay on the desktop.
use eframe::egui;

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
}

/// Trait defining the core behavior of the Kiwi GUI.
pub trait MascotRenderer {
    /// Initializes the GUI window and starts the rendering loop.
    /// TODO: Implement a transparent/overlay window setup in eframe.
    fn run(&mut self) -> Result<(), String>;

    /// Updates the mascot's state to trigger different animations or UI cues.
    fn set_state(&mut self, state: MascotState);
}

/// The main application struct for the Kiwi GUI.
pub struct KiwiGui {
    pub state: MascotState,
    // TODO: Add fields for animation frames, textures, or UI styling parameters.
}

impl KiwiGui {
    pub fn new() -> Self {
        Self {
            state: MascotState::Idle,
        }
    }
}

impl eframe::App for KiwiGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // TODO: Implement actual egui rendering logic here.
        // Example: Draw a parrot icon, show a bubble if speaking, etc.
        #[allow(deprecated)]
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Kiwi 🦜");
            ui.label(format!("State: {:?}", self.state));
        });
    }

    // Required trait method. No specific UI to push onto default context frame here.
    fn ui(&mut self, _ui: &mut egui::Ui, _frame: &mut eframe::Frame) {}
}

impl MascotRenderer for KiwiGui {
    fn run(&mut self) -> Result<(), String> {
        // TODO: Create eframe::NativeOptions with transparency and no decorations.
        // eframe::run_native(...)
        Ok(())
    }

    fn set_state(&mut self, state: MascotState) {
        self.state = state;
        // TODO: Trigger a repaint or animation state change.
    }
}
