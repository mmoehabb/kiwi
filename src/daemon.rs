use crate::agents::Orchestrator;
use crate::audio::{AudioManager, SpeechToText, TextToSpeech, WakeWordListener};
use crate::config::Configuration;
use crate::event::KiwiEvent;
use crate::interruption::InterruptionDetector;
use crate::wakeword::WakewordEngine;

use rodio::{OutputStream, Sink};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

async fn handle_farewell(
    orchestrator_arc: Arc<Mutex<Orchestrator>>,
    audio_mgr_clone: Arc<AudioManager>,
) -> String {
    let bye_response = {
        let orch = orchestrator_arc.lock().await;
        orch.process_farewell().await
    };

    match audio_mgr_clone.speak(&bye_response).await {
        Ok(audio_buffer) => {
            tokio::task::spawn_blocking(move || {
                let (_stream, stream_handle) = OutputStream::try_default().unwrap();
                let sink = Sink::try_new(&stream_handle).unwrap();
                let buffer = rodio::buffer::SamplesBuffer::new(1, 24000, audio_buffer);
                sink.append(buffer);
                sink.sleep_until_end();
            })
            .await
            .unwrap();
        }
        Err(e) => eprintln!("TTS Error for bye prompt: {}", e),
    }

    bye_response
}

#[allow(clippy::too_many_arguments)]
async fn handle_playback_with_interruption(
    audio_mgr_clone: Arc<AudioManager>,
    event_tx: mpsc::Sender<KiwiEvent>,
    orchestrator_arc: Arc<Mutex<Orchestrator>>,
    mut current_response: String,
) -> bool {
    loop {
        match audio_mgr_clone.speak(&current_response).await {
            Ok(audio_buffer) => {
                let _ = event_tx
                    .send(KiwiEvent::AssistantResponse(current_response.clone()))
                    .await;

                let detector = InterruptionDetector::new(0.02);

                let (stop_tx, mut stop_rx) = tokio::sync::mpsc::channel::<()>(1);
                let stop_tx_clone = stop_tx.clone();
                let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);

                tokio::task::spawn_blocking(move || {
                    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
                    let sink = Sink::try_new(&stream_handle).unwrap();
                    let buffer = rodio::buffer::SamplesBuffer::new(1, 24000, audio_buffer);
                    sink.append(buffer);

                    while !sink.empty() {
                        if stop_rx.try_recv().is_ok() {
                            sink.stop();
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                    let _ = stop_tx_clone.blocking_send(());
                });

                tokio::select! {
                    _ = stop_tx.closed() => {
                        let _ = cancel_tx.send(true);
                        return false;
                    }
                    res = detector.wait_for_interruption(cancel_rx) => {
                        println!("Interruption detected!");
                        let _ = stop_tx.send(()).await;

                        if let Ok((audio_data, input_sample_rate)) = res {
                            match audio_mgr_clone.transcribe_audio(audio_data, input_sample_rate).await {
                                Ok(interruption_text) => {
                                    println!("Interruption text: {}", interruption_text);
                                    if !interruption_text.is_empty() {
                                        let _ = event_tx
                                            .send(KiwiEvent::TranscribedText(interruption_text.clone()))
                                            .await;

                                        let mut orch = orchestrator_arc.lock().await;
                                        let (new_response, exit_conv) = orch.process_input(&interruption_text).await;

                                        if exit_conv {
                                            return true;
                                        }

                                        current_response = new_response;
                                        println!("Interruption Response: {}", current_response);
                                        continue;
                                    } else {
                                        return false;
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error transcribing interruption: {}", e);
                                    return false;
                                }
                            }
                        } else {
                            return false;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("TTS Error: {}", e);
                return false;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn run_background_daemon(
    audio_mgr_clone: Arc<AudioManager>,
    wakeword_engine_arc_clone: Arc<Mutex<WakewordEngine>>,
    config_daemon: Arc<Configuration>,
    event_tx: mpsc::Sender<KiwiEvent>,
    orchestrator: Orchestrator,
) {
    let orchestrator_arc = Arc::new(Mutex::new(orchestrator));
    println!("Background daemon started. Listening for wake word...");
    loop {
        if let Err(e) = audio_mgr_clone
            .wait_for_wake_word(wakeword_engine_arc_clone.clone())
            .await
        {
            eprintln!("Wake word error: {}", e);
            continue;
        }
        println!("Wake word detected!");
        let _ = event_tx.send(KiwiEvent::WakeWordDetected).await;

        let wake_response = {
            let orch = orchestrator_arc.lock().await;
            let wake_word_prompt = config_daemon.app.wake_word.clone();
            orch.speaker.generate_response(&wake_word_prompt).await
        };

        match audio_mgr_clone.speak(&wake_response).await {
            Ok(audio_buffer) => {
                let (_stream, stream_handle) = OutputStream::try_default().unwrap();
                let sink = Sink::try_new(&stream_handle).unwrap();
                let buffer = rodio::buffer::SamplesBuffer::new(1, 24000, audio_buffer);
                sink.append(buffer);
                sink.sleep_until_end();
            }
            Err(e) => eprintln!("TTS Error for wake prompt: {}", e),
        }

        'conversation: loop {
            let _ = event_tx.send(KiwiEvent::WakeWordDetected).await;

            match audio_mgr_clone.listen_and_transcribe().await {
                Ok(text) => {
                    if text.is_empty() {
                        println!("Conversation finished.");
                        handle_farewell(orchestrator_arc.clone(), audio_mgr_clone.clone()).await;
                        let _ = event_tx.send(KiwiEvent::Idle).await;
                        break 'conversation;
                    }

                    println!("Heard: {}", text);
                    let _ = event_tx
                        .send(KiwiEvent::TranscribedText(text.clone()))
                        .await;

                    let (response, exit_conv) = {
                        let mut orch = orchestrator_arc.lock().await;
                        orch.process_input(&text).await
                    };

                    if exit_conv {
                        handle_farewell(orchestrator_arc.clone(), audio_mgr_clone.clone()).await;
                        let _ = event_tx.send(KiwiEvent::Idle).await;
                        break 'conversation;
                    }
                    println!("Response: {}", response);

                    let exit_after_playback = handle_playback_with_interruption(
                        audio_mgr_clone.clone(),
                        event_tx.clone(),
                        orchestrator_arc.clone(),
                        response,
                    )
                    .await;

                    if exit_after_playback {
                        handle_farewell(orchestrator_arc.clone(), audio_mgr_clone.clone()).await;
                        let _ = event_tx.send(KiwiEvent::Idle).await;
                        break 'conversation;
                    }
                }
                Err(e) => {
                    eprintln!("STT Error: {}", e);
                    break 'conversation;
                }
            }
        }
        println!("Returning to idle state.");
    }
}
