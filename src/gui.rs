use kiwi_core::gui::{MascotRenderer, MascotState};

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
