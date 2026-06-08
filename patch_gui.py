import sys

def modify_gui():
    with open("src/gui.rs", "r") as f:
        content = f.read()

    content = content.replace(
        "    Speaking,\n}",
        "    Speaking,\n    /// Mascot is in onboarding state, prompting user to record wakeword.\n    Onboarding,\n}"
    )

    new_struct_fields = """    position_set: bool,

    // Onboarding UI state
    pub templates_recorded: usize,
    pub is_recording: bool,
    pub tx_gui: Option<mpsc::Sender<GuiEvent>>,
}

#[derive(Debug, Clone)]
pub enum GuiEvent {
    RecordSample,
    DoneOnboarding,
}
"""
    content = content.replace("    position_set: bool,\n}", new_struct_fields)

    new_new_method = """    pub fn new(rx: mpsc::Receiver<MascotState>, tx_gui: mpsc::Sender<GuiEvent>) -> Self {
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
    }"""
    content = content.replace("    pub fn new(rx: mpsc::Receiver<MascotState>) -> Self {\n        Self {\n            state: MascotState::Idle,\n            rx,\n            idle_texture: None,\n            listening_texture: None,\n            thinking_texture: None,\n            speaking_texture: None,\n            position_set: false,\n        }\n    }", new_new_method)

    new_match = """        let texture = match self.state {
            MascotState::Idle | MascotState::Onboarding => &self.idle_texture,
            MascotState::Listening => &self.listening_texture,
            MascotState::Thinking => &self.thinking_texture,
            MascotState::Speaking => &self.speaking_texture,
        };"""
    content = content.replace("""        let texture = match self.state {
            MascotState::Idle => &self.idle_texture,
            MascotState::Listening => &self.listening_texture,
            MascotState::Thinking => &self.thinking_texture,
            MascotState::Speaking => &self.speaking_texture,
        };""", new_match)

    new_ui_logic = """        if let Some(tex) = texture {
            ui.add(egui::Image::new(tex));
        } else {
            // Fallback text if images fail to load
            ui.heading("Kiwi 🦜");
            ui.label(format!("State: {:?}", self.state));
        }

        if self.state == MascotState::Onboarding {
            egui::Window::new("Welcome to Kiwi!")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label("Kiwi needs to learn your wake word.");
                    ui.label(format!("Samples recorded: {}/3", self.templates_recorded));

                    if self.is_recording {
                        ui.label("Recording... Please speak your wake word.");
                    } else if self.templates_recorded < 3 {
                        if ui.button("Record Sample").clicked() {
                            self.is_recording = true;
                            if let Some(tx) = &self.tx_gui {
                                let _ = tx.blocking_send(GuiEvent::RecordSample);
                            }
                        }
                    } else {
                        if ui.button("Done").clicked() {
                            if let Some(tx) = &self.tx_gui {
                                let _ = tx.blocking_send(GuiEvent::DoneOnboarding);
                            }
                        }
                    }
                });
        }"""
    content = content.replace("""        if let Some(tex) = texture {
            ui.add(egui::Image::new(tex));
        } else {
            // Fallback text if images fail to load
            ui.heading("Kiwi 🦜");
            ui.label(format!("State: {:?}", self.state));
        }""", new_ui_logic)

    with open("src/gui.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify_gui()
