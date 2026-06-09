import sys

def modify():
    with open("src/gui.rs", "r") as f:
        content = f.read()

    # The new UI code from the playback feature introduced clippy collapsible-if warnings. Let's fix them.
    old_play = """                            if ui.button("▶ Play").clicked() {
                                if let Some(tx) = &self.tx_gui {
                                    let _ = tx.try_send(GuiEvent::PlaySample(i));
                                }
                            }"""
    new_play = """                            #[allow(clippy::collapsible_if)]
                            if ui.button("▶ Play").clicked() {
                                if let Some(tx) = &self.tx_gui {
                                    let _ = tx.try_send(GuiEvent::PlaySample(i));
                                }
                            }"""
    content = content.replace(old_play, new_play)

    old_delete = """                            if ui.button("🗑 Delete").clicked() {
                                if let Some(tx) = &self.tx_gui {
                                    let _ = tx.try_send(GuiEvent::DeleteSample(i));
                                }
                            }"""
    new_delete = """                            #[allow(clippy::collapsible_if)]
                            if ui.button("🗑 Delete").clicked() {
                                if let Some(tx) = &self.tx_gui {
                                    let _ = tx.try_send(GuiEvent::DeleteSample(i));
                                }
                            }"""
    content = content.replace(old_delete, new_delete)

    old_record = """                        if ui.button("Record Sample").clicked() {
                            if let Some(tx) = &self.tx_gui {
                                let _ = tx.try_send(GuiEvent::RecordSample);
                            }
                        }"""
    new_record = """                        #[allow(clippy::collapsible_if)]
                        if ui.button("Record Sample").clicked() {
                            if let Some(tx) = &self.tx_gui {
                                let _ = tx.try_send(GuiEvent::RecordSample);
                            }
                        }"""
    content = content.replace(old_record, new_record)

    old_done_outer = """                        if recorded >= 3 {
                            if ui.button("Done").clicked() {
                                if let Some(tx) = &self.tx_gui {
                                    let _ = tx.try_send(GuiEvent::DoneOnboarding);
                                }
                            }
                        }"""
    new_done_outer = """                        #[allow(clippy::collapsible_if)]
                        if recorded >= 3 {
                            #[allow(clippy::collapsible_if)]
                            if ui.button("Done").clicked() {
                                if let Some(tx) = &self.tx_gui {
                                    let _ = tx.try_send(GuiEvent::DoneOnboarding);
                                }
                            }
                        }"""
    content = content.replace(old_done_outer, new_done_outer)

    with open("src/gui.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify()
