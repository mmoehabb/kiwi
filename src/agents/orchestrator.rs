use crate::agents::explorer::Explorer;
use crate::agents::speaker::Speaker;
use crate::agents::supervisor::Supervisor;
use crate::agents::thinker::Thinker;
use crate::intent::Intent;
use crate::llm::LocalLlm;
use crate::memory::ContextManager;
use crate::monitor::AgentsFlowMonitor;
use crate::permissions::PermissionManager;
use std::sync::Arc;

pub struct Orchestrator {
    _llm: Arc<LocalLlm>,
    pub speaker: Speaker,
    explorer: Explorer,
    thinker: Thinker,
    pub supervisor: Supervisor,
    perm_manager: Arc<PermissionManager>,
    monitor: AgentsFlowMonitor,
}

impl Orchestrator {
    pub fn new(
        llm: Arc<LocalLlm>,
        speaker: Speaker,
        explorer: Explorer,
        thinker: Thinker,
        supervisor: Supervisor,
        perm_manager: Arc<PermissionManager>,
        monitor: AgentsFlowMonitor,
    ) -> Self {
        Self {
            _llm: llm,
            speaker,
            explorer,
            thinker,
            supervisor,
            perm_manager,
            monitor,
        }
    }

    pub async fn process_farewell(&self) -> String {
        self.speaker.generate_response("bye").await
    }

    pub async fn process_input(&mut self, text: &str) -> (String, bool) {
        self.monitor.log("User Input", text);

        self.monitor.log(
            "orchestrator to supervisor (context)",
            &format!("Store user context: {}", text),
        );
        self.supervisor
            .store_context("user", text.to_string())
            .await;

        self.monitor.log(
            "orchestrator to thinker",
            &format!("What is the intent of '{}'", text),
        );
        let intent = match self.thinker.determine_intent(text).await {
            Ok(i) => {
                self.monitor.log("Thinker Response", &format!("{:?}", i));
                i
            }
            Err(e) => {
                self.monitor
                    .log("Thinker Response", &format!("Error: {}", e));
                eprintln!("Intent routing error: {}", e);
                Intent::Chat
            }
        };

        println!("Intent: {:?}", intent);

        let mut web_recap = String::new();
        let mut exit_conversation = false;

        match intent {
            Intent::Chat => {}
            Intent::SearchRequired { query } => {
                self.monitor.log(
                    "orchestrator to explorer",
                    &format!("Fetch info for query '{}'", query),
                );
                match self.explorer.fetch_info(&query).await {
                    Ok(recap) => {
                        self.monitor.log("Explorer Response", &recap);
                        web_recap = format!(
                            "System Note: The following is the latest information fetched from the web regarding '{}':\n{}\n\n",
                            query, recap
                        );
                    }
                    Err(e) => {
                        self.monitor
                            .log("Explorer Response", &format!("Error: {}", e));
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

                self.monitor.log("orchestrator to thinker", &ask_prompt);
                let ask_yes_no_response = self.thinker.ask_yes_no(&ask_prompt).await;
                self.monitor
                    .log("Thinker Response", &format!("{}", ask_yes_no_response));

                if !ask_yes_no_response {
                    let current_keywords = self.supervisor.extract_keywords(text).await;
                    let all_last_entries_relevant = self.supervisor.evaluate_relevance(text).await;
                    let mut context_prompt = self
                        .supervisor
                        .memory_bank
                        .build_prompt(&current_keywords, &all_last_entries_relevant);

                    if context_prompt.ends_with("<|start_header_id|>assistant<|end_header_id|>\n\n")
                    {
                        context_prompt.truncate(
                            context_prompt.len()
                                - "<|start_header_id|>assistant<|end_header_id|>\n\n".len(),
                        );
                    }

                    self.monitor.log(
                        "orchestrator to thinker",
                        &format!(
                            "Generate search query for '{}' with context '{}'",
                            text, context_prompt
                        ),
                    );
                    let search_query = self
                        .thinker
                        .generate_search_query(text, &context_prompt)
                        .await;
                    self.monitor.log("Thinker Response", &search_query);

                    if !search_query.is_empty() {
                        self.monitor.log(
                            "orchestrator to explorer",
                            &format!("Fetch info for query '{}'", search_query),
                        );
                        match self.explorer.fetch_info(&search_query).await {
                            Ok(recap) => {
                                self.monitor.log("Explorer Response", &recap);
                                web_recap = format!(
                                    "System Note: The following is the latest information fetched from the web regarding '{}':\n{}\n\n",
                                    search_query, recap
                                );
                            }
                            Err(e) => {
                                self.monitor
                                    .log("Explorer Response", &format!("Error: {}", e));
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
            Intent::ExecuteCommand { command } => match self.perm_manager.execute(&command) {
                Ok(_) => {
                    self.monitor.log(
                        "orchestrator to supervisor",
                        &format!("Store system message for executing command: {}", command),
                    );
                    self.supervisor
                        .store_system_message(format!("Successfully executed command: {}", command))
                        .await;
                }
                Err(e) => {
                    self.monitor.log(
                        "orchestrator to supervisor",
                        &format!("Store system message for failed command: {}", command),
                    );
                    self.supervisor
                        .store_system_message(format!(
                            "Failed to execute command '{}': {}",
                            command, e
                        ))
                        .await;
                }
            },
            Intent::StoreMemory { content, keywords } => {
                self.monitor.log(
                    "orchestrator to supervisor",
                    &format!("Store memory: {}", content),
                );
                self.supervisor.store_memory(content, keywords).await;
            }
            Intent::Farewell => {
                exit_conversation = true;
            }
        }

        if exit_conversation {
            // Early return to allow caller to handle farewell properly
            return (String::new(), exit_conversation);
        }

        self.monitor.log(
            "orchestrator to supervisor",
            "Extract keywords and evaluate relevance",
        );
        let current_keywords = self.supervisor.extract_keywords(text).await;
        let all_last_entries_relevant = self.supervisor.evaluate_relevance(text).await;
        self.monitor.log(
            "Supervisor Response",
            &format!(
                "Keywords: {:?}, Relevance: {:?}",
                current_keywords, all_last_entries_relevant
            ),
        );

        let mut prompt = self
            .supervisor
            .memory_bank
            .build_prompt(&current_keywords, &all_last_entries_relevant);

        if prompt.ends_with("<|start_header_id|>assistant<|end_header_id|>\n\n") {
            prompt
                .truncate(prompt.len() - "<|start_header_id|>assistant<|end_header_id|>\n\n".len());
        }

        prompt.push_str(&format!(
            "<|start_header_id|>user<|end_header_id|>\n\n{}{}<|eot_id|><|start_header_id|>assistant<|end_header_id|>\n\n",
            web_recap, text
        ));

        self.monitor.log("orchestrator to speaker", &prompt);
        let final_response = self.speaker.generate_response(&prompt).await;
        self.monitor.log("Speaker Response", &final_response);

        self.monitor.log(
            "orchestrator to supervisor (context)",
            &format!("Store assistant context: {}", final_response),
        );
        self.supervisor
            .store_context("assistant", final_response.clone())
            .await;

        (final_response, exit_conversation)
    }
}
