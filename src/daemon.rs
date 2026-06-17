use crate::audio::{AudioManager, SpeechToText, TextToSpeech, WakeWordListener};
use crate::config::Configuration;
use crate::event::KiwiEvent;
use crate::intent::{Intent, IntentRouter, LlmIntentRouter};
use crate::interruption::InterruptionDetector;
use crate::llm::{LlmEngine, LocalLlm};
use crate::memory::{ContextManager, MemoryBank, Message};
use crate::permissions::PermissionManager;
use crate::wakeword::WakewordEngine;
use crate::web::WebTool;
use rodio::{OutputStream, Sink};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

async fn handle_farewell(llm_clone: Arc<LocalLlm>, audio_mgr_clone: Arc<AudioManager>) -> String {
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
            })
            .await
            .unwrap();
        }
        Err(e) => eprintln!("TTS Error: {}", e),
    }
    bye_response
}

async fn process_intent(
    intent: Intent,
    text: &str,
    web_tool: &WebTool,
    llm_clone: Arc<LocalLlm>,
    perm_manager: &PermissionManager,
    memory_bank: &mut MemoryBank,
    audio_mgr_clone: Arc<AudioManager>,
) -> (String, bool) {
    let mut web_recap = String::new();
    let mut exit_conversation = false;

    match intent {
        Intent::Chat => {
            // We no longer store standard conversational back-and-forth
        }
        Intent::SearchRequired { query } => match web_tool.search_and_recap(&query).await {
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
        },
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
        Intent::ExecuteCommand { command } => match perm_manager.execute(&command) {
            Ok(_) => {
                let _ = memory_bank
                    .add_message(Message {
                        role: "system".to_string(),
                        content: format!("Successfully executed command: {}", command),
                        keywords: None,
                    })
                    .await;
            }
            Err(e) => {
                let _ = memory_bank
                    .add_message(Message {
                        role: "system".to_string(),
                        content: format!("Failed to execute command '{}': {}", command, e),
                        keywords: None,
                    })
                    .await;
            }
        },
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
            handle_farewell(llm_clone.clone(), audio_mgr_clone.clone()).await;
            exit_conversation = true;
        }
    }

    (web_recap, exit_conversation)
}

async fn generate_response(
    llm_clone: Arc<LocalLlm>,
    memory_bank: &MemoryBank,
    text: &str,
    web_recap: &str,
) -> String {
    let current_keywords = llm_clone.extract_keywords(text).await.unwrap_or_default();

    let history_len = memory_bank.history.len();
    let last_five_start = history_len.saturating_sub(5);

    let mut recent_entries = Vec::new();
    for (i, msg) in memory_bank.history.iter().enumerate() {
        if i >= last_five_start && msg.content != MemoryBank::SYSTEM_PROMPT {
            recent_entries.push(msg.content.clone());
        }
    }

    let mut relevant_last_entries = Vec::new();

    if !recent_entries.is_empty() {
        let mut evaluation_prompt = String::from(
            "For each of the following messages, determine if it shares the same topic as the current user query.\n\
             Current user query: \"",
        );
        evaluation_prompt.push_str(text);
        evaluation_prompt.push_str("\"\n\nMessages:\n");

        for (i, entry) in recent_entries.iter().enumerate() {
            evaluation_prompt.push_str(&format!("{}. \"{}\"\n", i + 1, entry));
        }

        evaluation_prompt.push_str(
            "\nReply with a comma-separated list of 'Yes' or 'No' for each message in order. \
             Example output: Yes, No, Yes",
        );

        let eval_response = llm_clone
            .generate(&evaluation_prompt)
            .await
            .unwrap_or_default();

        let answers: Vec<&str> = eval_response.split(',').map(|s| s.trim()).collect();
        for answer in answers {
            let answer_lower = answer.to_lowercase();
            // remove punctuation
            let cleaned_answer = answer_lower.trim_matches(|c: char| !c.is_alphabetic());
            if cleaned_answer == "yes" {
                relevant_last_entries.push(true);
            } else {
                relevant_last_entries.push(false);
            }
        }

        // Pad with false if LLM returned too few answers
        while relevant_last_entries.len() < recent_entries.len() {
            relevant_last_entries.push(false);
        }
        // Truncate if LLM returned too many
        relevant_last_entries.truncate(recent_entries.len());
    }

    let mut all_last_entries_relevant = Vec::new();
    let mut idx = 0;
    for (i, msg) in memory_bank.history.iter().enumerate() {
        if i >= last_five_start {
            if msg.content == MemoryBank::SYSTEM_PROMPT {
                all_last_entries_relevant.push(true); // Doesn't matter, handled inside build_prompt
            } else {
                if idx < relevant_last_entries.len() {
                    all_last_entries_relevant.push(relevant_last_entries[idx]);
                    idx += 1;
                } else {
                    all_last_entries_relevant.push(false);
                }
            }
        }
    }

    let mut prompt = memory_bank.build_prompt(&current_keywords, &all_last_entries_relevant);

    if prompt.ends_with("<|start_header_id|>assistant<|end_header_id|>\n\n") {
        prompt.truncate(prompt.len() - "<|start_header_id|>assistant<|end_header_id|>\n\n".len());
    }

    prompt.push_str(&format!(
        "<|start_header_id|>user<|end_header_id|>\n\n{}{}<|eot_id|><|start_header_id|>assistant<|end_header_id|>\n\n",
        web_recap, text
    ));

    match llm_clone.generate(&prompt).await {
        Ok(res) => res,
        Err(e) => format!("Error generating response: {}", e),
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_playback_with_interruption(
    audio_mgr_clone: Arc<AudioManager>,
    event_tx: mpsc::Sender<KiwiEvent>,
    llm_clone: Arc<LocalLlm>,
    memory_bank: &mut MemoryBank,
    web_tool: &WebTool,
    perm_manager: &PermissionManager,
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

                                        let intent_router = LlmIntentRouter::new(&*llm_clone);
                                        let intent = match intent_router.route_intent(&interruption_text).await {
                                            Ok(i) => i,
                                            Err(e) => {
                                                eprintln!("Intent routing error: {}", e);
                                                Intent::Chat
                                            }
                                        };

                                        println!("Interruption Intent: {:?}", intent);

                                        let (web_recap, exit_conv) = process_intent(
                                            intent,
                                            &interruption_text,
                                            web_tool,
                                            llm_clone.clone(),
                                            perm_manager,
                                            memory_bank,
                                            audio_mgr_clone.clone(),
                                        ).await;

                                        if exit_conv {
                                            return true;
                                        }

                                        current_response = generate_response(
                                            llm_clone.clone(),
                                            memory_bank,
                                            &interruption_text,
                                            &web_recap
                                        ).await;
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
    llm_daemon: Arc<LocalLlm>,
    event_tx: mpsc::Sender<KiwiEvent>,
    web_tool: WebTool,
    perm_manager: PermissionManager,
    mut memory_bank: MemoryBank,
) {
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
                        handle_farewell(llm_clone.clone(), audio_mgr_clone.clone()).await;
                        let _ = event_tx.send(KiwiEvent::Idle).await;
                        break 'conversation;
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

                    let (web_recap, exit_conv) = process_intent(
                        intent,
                        &text,
                        &web_tool,
                        llm_clone.clone(),
                        &perm_manager,
                        &mut memory_bank,
                        audio_mgr_clone.clone(),
                    )
                    .await;

                    if exit_conv {
                        break 'conversation;
                    }

                    let response =
                        generate_response(llm_clone.clone(), &memory_bank, &text, &web_recap).await;
                    println!("Response: {}", response);

                    let exit_after_playback = handle_playback_with_interruption(
                        audio_mgr_clone.clone(),
                        event_tx.clone(),
                        llm_clone.clone(),
                        &mut memory_bank,
                        &web_tool,
                        &perm_manager,
                        response,
                    )
                    .await;

                    if exit_after_playback {
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
