import sys

def modify():
    with open("src/gui.rs", "r") as f:
        content = f.read()

    old1 = """                        if ui.button("Record Sample").clicked() {
                            if let Some(tx) = &self.tx_gui {
                                let _ = tx.blocking_send(GuiEvent::RecordSample);
                            }
                        }"""
    new1 = """                        if ui.button("Record Sample").clicked() {
                            #[allow(clippy::collapsible_if)]
                            if let Some(tx) = &self.tx_gui {
                                let _ = tx.blocking_send(GuiEvent::RecordSample);
                            }
                        }"""
    content = content.replace(old1, new1)

    old2 = """                        if ui.button("Done").clicked() {
                            if let Some(tx) = &self.tx_gui {
                                let _ = tx.blocking_send(GuiEvent::DoneOnboarding);
                            }
                        }"""
    new2 = """                        #[allow(clippy::collapsible_if)]
                        if ui.button("Done").clicked() {
                            if let Some(tx) = &self.tx_gui {
                                let _ = tx.blocking_send(GuiEvent::DoneOnboarding);
                            }
                        }"""
    content = content.replace(old2, new2)

    with open("src/gui.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify()
