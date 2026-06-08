import sys

def modify():
    with open("src/gui.rs", "r") as f:
        content = f.read()

    # Change MascotState::Onboarding to hold state
    old_state = "    Onboarding,"
    new_state = "    Onboarding { recorded: usize, is_recording: bool },"
    content = content.replace(old_state, new_state)

    old_match = """        let texture = match self.state {
            MascotState::Idle | MascotState::Onboarding => &self.idle_texture,"""
    new_match = """        let texture = match self.state {
            MascotState::Idle | MascotState::Onboarding { .. } => &self.idle_texture,"""
    content = content.replace(old_match, new_match)

    old_ui = """        if self.state == MascotState::Onboarding {
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
                        #[allow(clippy::collapsible_if)]
                        if ui.button("Done").clicked() {
                            if let Some(tx) = &self.tx_gui {
                                let _ = tx.blocking_send(GuiEvent::DoneOnboarding);
                            }
                        }
                    }
                });
        }"""

    new_ui = """        if let MascotState::Onboarding { recorded, is_recording } = self.state {
            egui::Window::new("Welcome to Kiwi!")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label("Kiwi needs to learn your wake word.");
                    ui.label(format!("Samples recorded: {}/3", recorded));

                    if is_recording {
                        ui.label("Recording... Please speak your wake word.");
                    } else if recorded < 3 {
                        if ui.button("Record Sample").clicked() {
                            // Immediately send state update to ourselves locally?
                            // No, just trigger event and let main.rs handle state update
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

    content = content.replace(old_ui, new_ui)

    with open("src/gui.rs", "w") as f:
        f.write(content)

    with open("src/main.rs", "r") as f:
        content = f.read()

    # Update MascotState::Onboarding to MascotState::Onboarding { recorded: 0, is_recording: false }
    content = content.replace("MascotState::Onboarding", "MascotState::Onboarding { recorded: 0, is_recording: false }")

    # We must update the state from main inside the spawn loop when RecordSample is triggered
    old_loop = """            while let Some(event) = gui_event_rx.recv().await {
                match event {
                    GuiEvent::RecordSample => {
                        let (audio_data, _rate) = tokio::task::spawn_blocking(|| {"""

    new_loop = """            let mut recorded = 0;
            while let Some(event) = gui_event_rx.recv().await {
                match event {
                    GuiEvent::RecordSample => {
                        let _ = gui_tx_clone.send(MascotState::Onboarding { recorded, is_recording: true }).await;
                        let (audio_data, _rate) = tokio::task::spawn_blocking(|| {"""
    content = content.replace(old_loop, new_loop)

    old_post_record = """                        let mut engine = wakeword_engine_arc_clone.lock().await;
                        engine.add_template(&processed);
                    }"""

    new_post_record = """                        let mut engine = wakeword_engine_arc_clone.lock().await;
                        engine.add_template(&processed);
                        recorded += 1;
                        let _ = gui_tx_clone.send(MascotState::Onboarding { recorded, is_recording: false }).await;
                    }"""
    content = content.replace(old_post_record, new_post_record)

    with open("src/main.rs", "w") as f:
        f.write(content)


if __name__ == "__main__":
    modify()
