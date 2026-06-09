import sys

def modify():
    with open("src/gui.rs", "r") as f:
        content = f.read()

    old_gui_event = """#[derive(Debug, Clone)]
pub enum GuiEvent {
    RecordSample,
    DoneOnboarding,
}"""
    new_gui_event = """#[derive(Debug, Clone)]
pub enum GuiEvent {
    RecordSample,
    DoneOnboarding,
    PlaySample(usize),
    DeleteSample(usize),
}"""
    content = content.replace(old_gui_event, new_gui_event)

    old_ui = """                    if is_recording {
                        ui.label("Recording... Please speak your wake word.");
                    } else if recorded < 3 {
                        if ui.button("Record Sample").clicked() {
                            #[allow(clippy::collapsible_if)]
                            if let Some(tx) = &self.tx_gui {
                                let _ = tx.try_send(GuiEvent::RecordSample);
                            }
                        }
                    } else {
                        #[allow(clippy::collapsible_if)]
                        if ui.button("Done").clicked() {
                            if let Some(tx) = &self.tx_gui {
                                let _ = tx.try_send(GuiEvent::DoneOnboarding);
                            }
                        }
                    }"""

    new_ui = """                    for i in 0..recorded {
                        ui.horizontal(|ui| {
                            ui.label(format!("Sample {}", i + 1));
                            if ui.button("▶ Play").clicked() {
                                if let Some(tx) = &self.tx_gui {
                                    let _ = tx.try_send(GuiEvent::PlaySample(i));
                                }
                            }
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
                        if ui.button("Record Sample").clicked() {
                            if let Some(tx) = &self.tx_gui {
                                let _ = tx.try_send(GuiEvent::RecordSample);
                            }
                        }
                        if recorded >= 3 {
                            if ui.button("Done").clicked() {
                                if let Some(tx) = &self.tx_gui {
                                    let _ = tx.try_send(GuiEvent::DoneOnboarding);
                                }
                            }
                        }
                    }"""
    content = content.replace(old_ui, new_ui)

    with open("src/gui.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify()
