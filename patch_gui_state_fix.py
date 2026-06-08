import sys

def modify():
    with open("src/gui.rs", "r") as f:
        content = f.read()

    # I failed to replace the UI correctly because the previous patch's `old_ui` was slightly mismatched.

    # We will slice properly.
    start_idx = content.find('        if self.state == MascotState::Onboarding {')
    end_idx = content.find('                });\n        }', start_idx) + len('                });\n        }')

    if start_idx != -1 and end_idx != -1:
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
        content = content[:start_idx] + new_ui + content[end_idx:]

    with open("src/gui.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify()
