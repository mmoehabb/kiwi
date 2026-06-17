use kiwi::audio::{AudioManager, SpeechToText, TextToSpeech, WakeWordListener};
use kiwi::config::Configuration;
use kiwi::event::KiwiEvent;
use kiwi::gui::{GuiEvent, KiwiGui, MascotState};
use kiwi::intent::{Intent, IntentRouter, LlmIntentRouter};
use kiwi::interruption::InterruptionDetector;
use kiwi::llm::{LlmEngine, LocalLlm};
use kiwi::memory::{ContextManager, MemoryBank, Message};
use kiwi::permissions::PermissionManager;
use kiwi::wakeword::WakewordEngine;
use kiwi::web::{WebClient, WebTool};
use rodio::{OutputStream, Sink};
use std::sync::Arc;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Kiwi...");

    let config = Arc::new(Configuration::new());
    let wakeword_path = Configuration::wakeword_templates_path().unwrap();
    let wakeword_engine = WakewordEngine::new(wakeword_path, config.app.wake_word_sensitivity);
    let wakeword_engine_arc = Arc::new(tokio::sync::Mutex::new(wakeword_engine));

    println!("Initializing LLM Engine with Ollama...");
    let mut llm = LocalLlm::new(config.clone());
    llm.load_model("", "").await?;
    let llm = Arc::new(llm);

    let audio_mgr = Arc::new(AudioManager::new(config.clone()).await?);

    let (event_tx, mut event_rx) = mpsc::channel::<KiwiEvent>(32);
    let (gui_tx, gui_rx) = mpsc::channel::<MascotState>(32);
    let audio_mgr_clone = audio_mgr.clone();

    let mut memory_bank = MemoryBank::new(2048)
        .await
        .expect("Failed to initialize memory bank");

    let web_client = Arc::new(WebClient::new(config.clone()));
    let web_tool = WebTool::new(web_client.clone(), llm.clone());

    let perm_manager = PermissionManager::load().unwrap_or_else(|_| {
        kiwi::permissions::PermissionManager::from_file(std::path::Path::new("/dev/null"))
            .unwrap_or_else(|_| {
                let mut p = std::env::temp_dir();
                p.push("kiwi_empty_perms.toml");
                std::fs::write(&p, "").unwrap_or_default();
                kiwi::permissions::PermissionManager::from_file(&p).unwrap()
            })
    });

    let (gui_event_tx, mut gui_event_rx) = tokio::sync::mpsc::channel(10);
    let gui_event_tx_clone = gui_event_tx.clone();
    let wakeword_engine_arc_clone = wakeword_engine_arc.clone();
    let gui_tx_clone = gui_tx.clone();
    let llm_daemon = llm.clone();
    let config_daemon = config.clone();
    tokio::spawn(async move {
        let has_templates = {
            let engine: tokio::sync::MutexGuard<kiwi::wakeword::WakewordEngine> =
                wakeword_engine_arc_clone.lock().await;
            engine.has_templates()
        };

        if !has_templates {
            let _ = gui_tx_clone
                .send(MascotState::Onboarding {
                    recorded: 0,
                    is_recording: false,
                })
                .await;
            let mut recorded = 0;
            let mut cached_raw_audio: Vec<Vec<f32>> = Vec::new();
            while let Some(event) = gui_event_rx.recv().await {
                match event {
                    GuiEvent::RecordSample => {
                        let _ = gui_tx_clone
                            .send(MascotState::Onboarding {
                                recorded,
                                is_recording: true,
                            })
                            .await;
                        let (audio_data, _rate) = tokio::task::spawn_blocking(|| {
                            let host = cpal::default_host();
                            use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
                            let device = host.default_input_device().unwrap();
                            let conf = device.default_input_config().unwrap();
                            let channels = conf.channels();
                            let rb = ringbuf::HeapRb::<f32>::new(conf.sample_rate().0 as usize * 5);
                            let (mut prod, mut cons) = ringbuf::traits::Split::split(rb);
                            let stream = match conf.sample_format() {
                                cpal::SampleFormat::F32 => device
                                    .build_input_stream(
                                        &conf.clone().into(),
                                        move |data: &[f32], _| {
                                            for frame in data.chunks(channels as usize) {
                                                let mono_sample =
                                                    frame.iter().sum::<f32>() / channels as f32;
                                                let _ = ringbuf::traits::Producer::try_push(
                                                    &mut prod,
                                                    mono_sample,
                                                );
                                            }
                                        },
                                        |err| eprintln!("error: {}", err),
                                        None,
                                    )
                                    .unwrap(),
                                _ => panic!("Unsupported format"),
                            };
                            stream.play().unwrap();
                            std::thread::sleep(std::time::Duration::from_millis(2000));
                            stream.pause().unwrap();
                            let mut buf = Vec::new();
                            while let Some(s) = ringbuf::traits::Consumer::try_pop(&mut cons) {
                                buf.push(s);
                            }
                            (buf, conf.sample_rate().0)
                        })
                        .await
                        .unwrap();
                        let processed = if _rate != 16000 {
                            use dasp::{Signal, interpolate::linear::Linear, signal};
                            let mut sig = signal::from_iter(audio_data.clone());
                            let interp = Linear::new(sig.next(), sig.next());
                            sig.from_hz_to_hz(interp, _rate as f64, 16000.0)
                                .take((audio_data.len() as f64 * (16000.0 / _rate as f64)) as usize)
                                .collect()
                        } else {
                            audio_data
                        };

                        cached_raw_audio.push(processed.clone());
                        let mut engine: tokio::sync::MutexGuard<kiwi::wakeword::WakewordEngine> =
                            wakeword_engine_arc_clone.lock().await;
                        engine.add_template(&processed);
                        recorded += 1;
                        let _ = gui_tx_clone
                            .send(MascotState::Onboarding {
                                recorded,
                                is_recording: false,
                            })
                            .await;
                    }
                    GuiEvent::PlaySample(idx) => {
                        if idx < cached_raw_audio.len() {
                            let audio = cached_raw_audio[idx].clone();
                            tokio::task::spawn_blocking(move || {
                                let (_stream, stream_handle) =
                                    rodio::OutputStream::try_default().unwrap();
                                let sink = rodio::Sink::try_new(&stream_handle).unwrap();
                                let buffer = rodio::buffer::SamplesBuffer::new(1, 16000, audio);
                                sink.append(buffer);
                                sink.sleep_until_end();
                            });
                        }
                    }
                    GuiEvent::DeleteSample(idx) => {
                        if idx < cached_raw_audio.len() {
                            cached_raw_audio.remove(idx);
                            let mut engine: tokio::sync::MutexGuard<
                                kiwi::wakeword::WakewordEngine,
                            > = wakeword_engine_arc_clone.lock().await;
                            engine.remove_template(idx);
                            recorded -= 1;
                            let _ = gui_tx_clone
                                .send(MascotState::Onboarding {
                                    recorded,
                                    is_recording: false,
                                })
                                .await;
                        }
                    }
                    GuiEvent::DoneOnboarding => {
                        let engine = wakeword_engine_arc_clone.lock().await;
                        let _ = engine.save_templates();
                        let _ = gui_tx_clone.send(MascotState::Idle).await;
                        break;
                    }
                }
            }
        }

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

            let wake_word_prompt = config_daemon.app.wake_word.clone();
            let wake_response = match llm_daemon.generate(&wake_word_prompt).await {
                Ok(res) => res,
                Err(e) => panic!("Error generating response for wake word: {}", e),
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

            let llm_clone = llm_daemon.clone();

            'conversation: loop {
                let _ = event_tx.send(KiwiEvent::WakeWordDetected).await;

                match audio_mgr_clone.listen_and_transcribe().await {
                    Ok(text) => {
                        if text.is_empty() {
                            println!("Conversation finished.");

                            let bye_response = match llm_clone.generate("bye").await {
                                Ok(res) => res,
                                Err(e) => panic!("Error generating response for bye: {}", e),
                            };

                            match audio_mgr_clone.speak(&bye_response).await {
                                Ok(audio_buffer) => {
                                    let (_stream, stream_handle) =
                                        OutputStream::try_default().unwrap();
                                    let sink = Sink::try_new(&stream_handle).unwrap();
                                    let buffer =
                                        rodio::buffer::SamplesBuffer::new(1, 24000, audio_buffer);
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

                        let intent_router = LlmIntentRouter::new(&*llm_clone);
                        let intent = match intent_router.route_intent(&text).await {
                            Ok(i) => i,
                            Err(e) => {
                                eprintln!("Intent routing error: {}", e);
                                Intent::Chat
                            }
                        };

                        println!("Intent: {:?}", intent);

                        let mut web_recap = String::new();

                        match intent {
                            Intent::Chat => {
                                // We no longer store standard conversational back-and-forth
                            }
                            Intent::SearchRequired { query } => {
                                match web_tool.search_and_recap(&query).await {
                                    Ok(recap) => {
                                        web_recap = format!(
                                            "System Note: The following is the latest information fetched from the web regarding '{}':\n{}\n\n",
                                            query, recap
                                        );
                                    }
                                    Err(e) => {
                                        eprintln!("Web search error: {}", e);
                                        web_recap = format!(
                                            "System Note: A web search was attempted but failed with error: {}\n\n",
                                            e
                                        );
                                    }
                                }
                            }
                            Intent::Inquiry => {
                                let ask_prompt = format!(
                                    "Does the system have the latest information to answer this user query: '{}'? Reply only 'Yes' or 'No'.",
                                    text
                                );
                                let has_latest = llm_clone
                                    .generate(&ask_prompt)
                                    .await
                                    .unwrap_or_default()
                                    .trim()
                                    .to_lowercase();

                                if has_latest.contains("no") {
                                    let query_prompt = format!(
                                        "Generate a short search query to find information about: '{}'. Output ONLY the query.",
                                        text
                                    );
                                    let search_query = llm_clone
                                        .generate(&query_prompt)
                                        .await
                                        .unwrap_or_default()
                                        .trim()
                                        .to_string();

                                    if !search_query.is_empty() {
                                        match web_tool.search_and_recap(&search_query).await {
                                            Ok(recap) => {
                                                web_recap = format!(
                                                    "System Note: The following is the latest information fetched from the web regarding '{}':\n{}\n\n",
                                                    search_query, recap
                                                );
                                            }
                                            Err(e) => {
                                                eprintln!("Web search error: {}", e);
                                                web_recap = format!(
                                                    "System Note: A web search was attempted but failed with error: {}\n\n",
                                                    e
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            Intent::ExecuteCommand { command } => {
                                match perm_manager.execute(&command) {
                                    Ok(_) => {
                                        let _ = memory_bank
                                            .add_message(Message {
                                                role: "system".to_string(),
                                                content: format!(
                                                    "Successfully executed command: {}",
                                                    command
                                                ),
                                                keywords: None,
                                            })
                                            .await;
                                    }
                                    Err(e) => {
                                        let _ = memory_bank
                                            .add_message(Message {
                                                role: "system".to_string(),
                                                content: format!(
                                                    "Failed to execute command '{}': {}",
                                                    command, e
                                                ),
                                                keywords: None,
                                            })
                                            .await;
                                    }
                                }
                            }
                            Intent::StoreMemory { content, keywords } => {
                                let _ = memory_bank
                                    .add_message(Message {
                                        role: "user".to_string(),
                                        content,
                                        keywords: Some(keywords),
                                    })
                                    .await;
                            }
                            Intent::Farewell => {
                                let bye_response = match llm_clone.generate("bye").await {
                                    Ok(res) => res,
                                    Err(e) => format!("Error generating response for bye: {}", e),
                                };

                                match audio_mgr_clone.speak(&bye_response).await {
                                    Ok(audio_buffer) => {
                                        tokio::task::spawn_blocking(move || {
                                            let (_stream, stream_handle) =
                                                OutputStream::try_default().unwrap();
                                            let sink = Sink::try_new(&stream_handle).unwrap();
                                            let buffer = rodio::buffer::SamplesBuffer::new(
                                                1,
                                                24000,
                                                audio_buffer,
                                            );
                                            sink.append(buffer);
                                            sink.sleep_until_end();
                                        })
                                        .await
                                        .unwrap();
                                    }
                                    Err(e) => eprintln!("TTS Error: {}", e),
                                }
                                break 'conversation;
                            }
                        }

                        // Extract keywords from the current text to find relevant memories
                        let current_keywords =
                            llm_clone.extract_keywords(&text).await.unwrap_or_default();

                        let mut prompt = memory_bank.build_prompt(&current_keywords);

                        // Because build_prompt ends with the assistant header, we should inject the user message before it.
                        if prompt.ends_with("<|start_header_id|>assistant<|end_header_id|>\n\n") {
                            prompt.truncate(
                                prompt.len()
                                    - "<|start_header_id|>assistant<|end_header_id|>\n\n".len(),
                            );
                        }

                        // Since we don't store standard chat, we must append the current user message to the prompt directly
                        prompt.push_str(&format!(
                            "<|start_header_id|>user<|end_header_id|>\n\n{}{}<|eot_id|><|start_header_id|>assistant<|end_header_id|>\n\n",
                            web_recap, text
                        ));

                        let response = match llm_clone.generate(&prompt).await {
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
                                            24000,
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

                                                            let intent_router = LlmIntentRouter::new(&*llm_clone);
                                                            let intent = match intent_router.route_intent(&interruption_text).await {
                                                                Ok(i) => i,
                                                                Err(e) => {
                                                                    eprintln!("Intent routing error: {}", e);
                                                                    Intent::Chat
                                                                }
                                                            };

                                                            println!("Interruption Intent: {:?}", intent);

                                                            let mut web_recap = String::new();

                                                            match intent {
                                                                Intent::Chat => {
                                                                    // We no longer store standard conversational back-and-forth
                                                                }
                                                                Intent::SearchRequired { query } => {
                                                                    match web_tool.search_and_recap(&query).await {
                                                                        Ok(recap) => {
                                                                            web_recap = format!(
                                                                                "System Note: The following is the latest information fetched from the web regarding '{}':\n{}\n\n",
                                                                                query, recap
                                                                            );
                                                                        }
                                                                        Err(e) => {
                                                                            eprintln!("Web search error: {}", e);
                                                                            web_recap = format!("System Note: A web search was attempted but failed with error: {}\n\n", e);
                                                                        }
                                                                    }
                                                                }
                                                                Intent::Inquiry => {
                                                                    let ask_prompt = format!(
                                                                        "Does the system have the latest information to answer this user query: '{}'? Reply only 'Yes' or 'No'.",
                                                                        interruption_text
                                                                    );
                                                                    let has_latest = llm_clone.generate(&ask_prompt).await.unwrap_or_default().trim().to_lowercase();

                                                                    if has_latest.contains("no") {
                                                                        let query_prompt = format!(
                                                                            "Generate a short search query to find information about: '{}'. Output ONLY the query.",
                                                                            interruption_text
                                                                        );
                                                                        let search_query = llm_clone.generate(&query_prompt).await.unwrap_or_default().trim().to_string();

                                                                        if !search_query.is_empty() {
                                                                            match web_tool.search_and_recap(&search_query).await {
                                                                                Ok(recap) => {
                                                                                    web_recap = format!(
                                                                                        "System Note: The following is the latest information fetched from the web regarding '{}':\n{}\n\n",
                                                                                        search_query, recap
                                                                                    );
                                                                                }
                                                                                Err(e) => {
                                                                                    eprintln!("Web search error: {}", e);
                                                                                    web_recap = format!("System Note: A web search was attempted but failed with error: {}\n\n", e);
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                Intent::ExecuteCommand { command } => {
                                                                    match perm_manager.execute(&command) {
                                                                        Ok(_) => {
                                                                            let _ = memory_bank
                                                                                .add_message(Message {
                                                                                    role: "system".to_string(),
                                                                                    content: format!(
                                                                                        "Successfully executed command: {}",
                                                                                        command
                                                                                    ),
                                                                                    keywords: None,
                                                                                })
                                                                                .await;
                                                                        }
                                                                        Err(e) => {
                                                                            let _ = memory_bank
                                                                                .add_message(Message {
                                                                                    role: "system".to_string(),
                                                                                    content: format!(
                                                                                        "Failed to execute command '{}': {}",
                                                                                        command, e
                                                                                    ),
                                                                                    keywords: None,
                                                                                })
                                                                                .await;
                                                                        }
                                                                    }
                                                                }
                                                                Intent::StoreMemory { content, keywords } => {
                                                                    let _ = memory_bank
                                                                        .add_message(Message {
                                                                            role: "user".to_string(),
                                                                            content,
                                                                            keywords: Some(keywords),
                                                                        })
                                                                        .await;
                                                                }
                                                                Intent::Farewell => {
                                                                    let bye_response = match llm_clone.generate("bye").await {
                                                                        Ok(res) => res,
                                                                        Err(e) => format!("Error generating response for bye: {}", e),
                                                                    };

                                                                    match audio_mgr_clone.speak(&bye_response).await {
                                                                        Ok(audio_buffer) => {
                                                                            tokio::task::spawn_blocking(move || {
                                                                                let (_stream, stream_handle) = OutputStream::try_default().unwrap();
                                                                                let sink = Sink::try_new(&stream_handle).unwrap();
                                                                                let buffer = rodio::buffer::SamplesBuffer::new(1, 24000, audio_buffer);
                                                                                sink.append(buffer);
                                                                                sink.sleep_until_end();
                                                                            }).await.unwrap();
                                                                        }
                                                                        Err(e) => eprintln!("TTS Error: {}", e),
                                                                    }
                                                                    break 'conversation;
                                                                }
                                                            }

                                                            let current_keywords = llm_clone.extract_keywords(&interruption_text).await.unwrap_or_default();
                                                            let mut prompt = memory_bank.build_prompt(&current_keywords);

                                                            if prompt.ends_with("<|start_header_id|>assistant<|end_header_id|>\n\n") {
                                                                prompt.truncate(prompt.len() - "<|start_header_id|>assistant<|end_header_id|>\n\n".len());
                                                            }

                                                            prompt.push_str(&format!(
                                                                "<|start_header_id|>user<|end_header_id|>\n\n{}{}<|eot_id|><|start_header_id|>assistant<|end_header_id|>\n\n",
                                                                web_recap, interruption_text
                                                            ));

                                                            current_response = match llm_clone.generate(&prompt).await {
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
        Box::new(move |_cc| Ok(Box::new(KiwiGui::new(gui_rx, gui_event_tx_clone)))),
    )?;

    Ok(())
}
