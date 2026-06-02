use kiwi::audio::{AudioManager, SpeechToText, TextToSpeech, WakeWordEngine};
use kiwi::config::Configuration;
use kiwi::event::KiwiEvent;
use kiwi::gui::{KiwiGui, MascotState};
use kiwi::llm::{LlmEngine, LocalLlm};
use rodio::{OutputStream, Sink};
use std::sync::Arc;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Kiwi...");

    let config = Arc::new(Configuration::new());

    println!("Initializing LLM Engine with Ollama...");
    let mut llm = LocalLlm::new(config.clone());
    llm.load_model("", "").await?;
    let llm = Arc::new(llm);

    let audio_mgr = Arc::new(AudioManager::new(config.clone()).await?);

    let (event_tx, mut event_rx) = mpsc::channel::<KiwiEvent>(32);
    let (gui_tx, gui_rx) = mpsc::channel::<MascotState>(32);
    let audio_mgr_clone = audio_mgr.clone();

    tokio::spawn(async move {
        println!("Background daemon started. Listening for wake word...");
        loop {
            if let Err(e) = audio_mgr_clone.wait_for_wake_word().await {
                eprintln!("Wake word error: {}", e);
                continue;
            }
            println!("Wake word detected!");
            let _ = event_tx.send(KiwiEvent::WakeWordDetected).await;

            match audio_mgr_clone.speak("How can I help you?").await {
                Ok(audio_buffer) => {
                    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
                    let sink = Sink::try_new(&stream_handle).unwrap();
                    let buffer = rodio::buffer::SamplesBuffer::new(1, 22050, audio_buffer);
                    sink.append(buffer);
                    sink.sleep_until_end();
                }
                Err(e) => eprintln!("TTS Error for wake prompt: {}", e),
            }

            let llm_clone = llm.clone();
            match audio_mgr_clone.listen_and_transcribe().await {
                Ok(text) => {
                    println!("Heard: {}", text);
                    let _ = event_tx
                        .send(KiwiEvent::TranscribedText(text.clone()))
                        .await;

                    let response = match llm_clone.generate(&text).await {
                        Ok(res) => res,
                        Err(e) => format!("Error generating response: {}", e),
                    };
                    println!("Response: {}", response);

                    match audio_mgr_clone.speak(&response).await {
                        Ok(audio_buffer) => {
                            let (_stream, stream_handle) = OutputStream::try_default().unwrap();
                            let sink = Sink::try_new(&stream_handle).unwrap();
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

            println!("Returning to idle state.");
        }
    });

    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            match event {
                KiwiEvent::WakeWordDetected => {
                    let _ = gui_tx.send(MascotState::Listening).await;
                }
                KiwiEvent::TranscribedText(_) => {
                    let _ = gui_tx.send(MascotState::Thinking).await;
                }
                KiwiEvent::AssistantResponse(_) => {
                    let _ = gui_tx.send(MascotState::Speaking).await;
                }
            }
        }
    });

    let options = eframe::NativeOptions {
        ..Default::default()
    };

    eframe::run_native(
        "Kiwi",
        options,
        Box::new(|_cc| Ok(Box::new(KiwiGui::new(gui_rx)))),
    )?;

    Ok(())
}
