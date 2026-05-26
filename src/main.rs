pub mod audio;
pub mod config;
pub mod gui;
pub mod llm;
pub mod memory;
pub mod plugin;
pub mod web;

use crate::audio::{AudioManager, SpeechToText, TextToSpeech, WakeWordEngine};
use crate::gui::KiwiGui;
use rodio::{OutputStream, Sink};
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum KiwiEvent {
    WakeWordDetected,
    TranscribedText(String),
    AssistantResponse(String),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🦜 Starting Kiwi...");

    let audio_mgr = Arc::new(AudioManager::new().await?);

    let (event_tx, mut event_rx) = mpsc::channel::<KiwiEvent>(32);
    let audio_mgr_clone = audio_mgr.clone();

    tokio::spawn(async move {
        println!("🦜 Background daemon started. Listening for wake word...");
        loop {
            if let Err(e) = audio_mgr_clone.wait_for_wake_word().await {
                eprintln!("Wake word error: {}", e);
                continue;
            }
            println!("🦜 Wake word detected!");
            let _ = event_tx.send(KiwiEvent::WakeWordDetected).await;

            match audio_mgr_clone.listen_and_transcribe().await {
                Ok(text) => {
                    println!("🦜 Heard: {}", text);
                    let _ = event_tx
                        .send(KiwiEvent::TranscribedText(text.clone()))
                        .await;

                    // Mock routing and LLM response for now
                    let response = format!("I heard you say: {}", text);

                    match audio_mgr_clone.speak(&response).await {
                        Ok(audio_buffer) => {
                            // Play the audio
                            let (_stream, stream_handle) = OutputStream::try_default().unwrap();
                            let sink = Sink::try_new(&stream_handle).unwrap();
                            // Assuming sample rate 22050 from Piper Lessac model
                            let buffer = rodio::buffer::SamplesBuffer::new(1, 22050, audio_buffer);
                            sink.append(buffer);
                            sink.sleep_until_end();
                        }
                        Err(e) => eprintln!("TTS Error: {}", e),
                    }
                    let _ = event_tx.send(KiwiEvent::AssistantResponse(response)).await;
                }
                Err(e) => {
                    eprintln!("STT Error: {}", e);
                }
            }

            println!("🦜 Returning to idle state.");
        }
    });

    // Event listener task
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            match event {
                KiwiEvent::WakeWordDetected => {
                    // TODO: Tell GUI to update to Listening
                }
                KiwiEvent::TranscribedText(_) => {
                    // TODO: Tell GUI to update to Thinking
                }
                KiwiEvent::AssistantResponse(_) => {
                    // TODO: Tell GUI to update to Speaking
                }
            }
        }
    });

    let options = eframe::NativeOptions {
        // TODO: Configure transparent, un-decorated window for the mascot.
        ..Default::default()
    };

    eframe::run_native(
        "Kiwi 🦜",
        options,
        Box::new(|_cc| Ok(Box::new(KiwiGui::new()))),
    )?;

    Ok(())
}
