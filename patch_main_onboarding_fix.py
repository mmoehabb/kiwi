import sys
import re

def modify():
    with open("src/main.rs", "r") as f:
        content = f.read()

    # The onboarding flow doesn't increment the counter or set `is_recording` back to false

    old_loop = """            let recorded = 0;
            while let Some(event) = gui_event_rx.recv().await {
                match event {
                    GuiEvent::RecordSample => {
                        let _ = gui_tx_clone
                            .send(MascotState::Onboarding {
                                recorded,
                                is_recording: true,
                            })
                            .await;
                        let (audio_data, _rate) = tokio::task::spawn_blocking(|| {"""

    new_loop = """            let mut recorded = 0;
            while let Some(event) = gui_event_rx.recv().await {
                match event {
                    GuiEvent::RecordSample => {
                        let _ = gui_tx_clone
                            .send(MascotState::Onboarding {
                                recorded,
                                is_recording: true,
                            })
                            .await;
                        let (audio_data, _rate) = tokio::task::spawn_blocking(|| {"""

    content = content.replace(old_loop, new_loop)

    old_post_record = """                        let mut engine: tokio::sync::MutexGuard<kiwi::wakeword::WakewordEngine> =
                            wakeword_engine_arc_clone.lock().await;
                        engine.add_template(&processed);
                    }"""

    new_post_record = """                        let mut engine: tokio::sync::MutexGuard<kiwi::wakeword::WakewordEngine> =
                            wakeword_engine_arc_clone.lock().await;
                        engine.add_template(&processed);
                        recorded += 1;
                        let _ = gui_tx_clone
                            .send(MascotState::Onboarding {
                                recorded,
                                is_recording: false,
                            })
                            .await;
                    }"""
    content = content.replace(old_post_record, new_post_record)

    with open("src/main.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify()
