use kiwi::audio::{AudioManager, SpeechToText, TextToSpeech, WakeWordEngine};
use kiwi::config::Configuration;
use kiwi::event::KiwiEvent;
use kiwi::gui::{KiwiGui, MascotState};
use kiwi::interruption::InterruptionDetector;
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

            loop {
                let _ = event_tx.send(KiwiEvent::WakeWordDetected).await;

                match audio_mgr_clone.listen_and_transcribe().await {
                    Ok(text) => {
                        if text.is_empty() {
                            println!("Conversation finished.");
                            match audio_mgr_clone
                                .speak("I'll be here when you need me.")
                                .await
                            {
                                Ok(audio_buffer) => {
                                    let (_stream, stream_handle) =
                                        OutputStream::try_default().unwrap();
                                    let sink = Sink::try_new(&stream_handle).unwrap();
                                    let buffer =
                                        rodio::buffer::SamplesBuffer::new(1, 22050, audio_buffer);
                                    sink.append(buffer);
                                    sink.sleep_until_end();
                                }
                                Err(e) => eprintln!("TTS Error: {}", e),
                            }
                            let _ = event_tx.send(KiwiEvent::Idle).await;
                            break;
                        }

                        println!("Heard: {}", text);
                        let _ = event_tx
                            .send(KiwiEvent::TranscribedText(text.clone()))
                            .await;

                        let response = match llm_clone.generate(&text).await {
                            Ok(res) => res,
                            Err(e) => format!("Error generating response: {}", e),
                        };
                        println!("Response: {}", response);

                        let mut current_response = response;

                        loop {
                            match audio_mgr_clone.speak(&current_response).await {
                                Ok(audio_buffer) => {
                                    let _ = event_tx
                                        .send(KiwiEvent::AssistantResponse(
                                            current_response.clone(),
                                        ))
                                        .await;

                                    let detector = InterruptionDetector::new(0.02); // Same threshold as silence

                                    // To make the sink cancelable, we must run the sleep on the current thread,
                                    // but we want to select on it. Alternatively, spawn a separate task to manage the rodio sink,
                                    // but since we want to be able to drop the sink, we do it in an async-friendly way using flume or similar,
                                    // or just use tokio tasks and pass the sink ownership so we can drop it.

                                    let (stop_tx, mut stop_rx) =
                                        tokio::sync::mpsc::channel::<()>(1);
                                    let stop_tx_clone = stop_tx.clone();

                                    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);

                                    tokio::task::spawn_blocking(move || {
                                        let (_stream, stream_handle) =
                                            OutputStream::try_default().unwrap();
                                        let sink = Sink::try_new(&stream_handle).unwrap();
                                        let buffer = rodio::buffer::SamplesBuffer::new(
                                            1,
                                            22050,
                                            audio_buffer,
                                        );
                                        sink.append(buffer);

                                        // Poll until sink is empty or we are told to stop
                                        while !sink.empty() {
                                            if stop_rx.try_recv().is_ok() {
                                                sink.stop();
                                                break;
                                            }
                                            std::thread::sleep(std::time::Duration::from_millis(
                                                50,
                                            ));
                                        }
                                        let _ = stop_tx_clone.blocking_send(());
                                    });

                                    tokio::select! {
                                        _ = stop_tx.closed() => {
                                            // Playback finished normally
                                            let _ = cancel_tx.send(true);
                                            break;
                                        }
                                        res = detector.wait_for_interruption(cancel_rx) => {
                                            println!("Interruption detected!");
                                            // Send stop signal and wait for it to be processed
                                            let _ = stop_tx.send(()).await;

                                            if let Ok((audio_data, input_sample_rate)) = res {
                                                match audio_mgr_clone.transcribe_audio(audio_data, input_sample_rate).await {
                                                    Ok(interruption_text) => {
                                                        println!("Interruption text: {}", interruption_text);
                                                        if !interruption_text.is_empty() {
                                                            let _ = event_tx
                                                                .send(KiwiEvent::TranscribedText(interruption_text.clone()))
                                                                .await;

                                                            current_response = match llm_clone.generate(&interruption_text).await {
                                                                Ok(res) => res,
                                                                Err(e) => format!("Error generating response: {}", e),
                                                            };
                                                            println!("Interruption Response: {}", current_response);
                                                            continue;
                                                        } else {
                                                            break;
                                                        }
                                                    }
                                                    Err(e) => {
                                                        eprintln!("Error transcribing interruption: {}", e);
                                                        break;
                                                    }
                                                }
                                            } else {
                                                break;
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("TTS Error: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("STT Error: {}", e);
                        break;
                    }
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
                KiwiEvent::Idle => {
                    let _ = gui_tx.send(MascotState::Idle).await;
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
