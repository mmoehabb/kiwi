import sys

def modify():
    with open("src/gui.rs", "r") as f:
        content = f.read()

    # Maybe we are missing an edge case where it renders a few frames while `is_recording` state is still processing its way through the mpsc channel to the gui task?
    # When you click the button, `tx.try_send(GuiEvent::RecordSample)` fires.
    # The next frame renders. Because `is_recording` is STILL false (the main task hasn't sent the Onboarding update yet), `clicked()` might not fire again (mouse is up), but what if it fires twice? No, `clicked()` is guaranteed to fire once.

    # Wait, eframe redraws continuously if requested. If we are clicking, the mouse might register multiple clicks if there is no debounce? No, `clicked()` handles debounce.

    # What if the user double clicks? We should probably update the LOCAL state to prevent another click before the backend confirms it.

    old_record = """                        #[allow(clippy::collapsible_if)]
                        if ui.button("Record Sample").clicked() {
                            if let Some(tx) = &self.tx_gui {
                                let _ = tx.try_send(GuiEvent::RecordSample);
                            }
                        }"""

    new_record = """                        #[allow(clippy::collapsible_if)]
                        if ui.button("Record Sample").clicked() {
                            if let Some(tx) = &self.tx_gui {
                                // Optimistically update state so we don't send multiple events
                                self.state = MascotState::Onboarding {
                                    recorded,
                                    is_recording: true,
                                };
                                let _ = tx.try_send(GuiEvent::RecordSample);
                            }
                        }"""

    content = content.replace(old_record, new_record)

    with open("src/gui.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify()
