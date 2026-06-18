use crate::agents::explorer::Explorer;
use crate::agents::speaker::Speaker;
use crate::agents::supervisor::Supervisor;
use crate::agents::thinker::Thinker;
use crate::intent::Intent;
use crate::llm::LocalLlm;
use crate::memory::ContextManager;
use crate::permissions::PermissionManager;
use std::sync::Arc;

pub struct Orchestrator {
    _llm: Arc<LocalLlm>,
    pub speaker: Speaker,
    explorer: Explorer,
    thinker: Thinker,
    pub supervisor: Supervisor,
    perm_manager: Arc<PermissionManager>,
}

impl Orchestrator {
    pub fn new(
        llm: Arc<LocalLlm>,
        speaker: Speaker,
        explorer: Explorer,
        thinker: Thinker,
        supervisor: Supervisor,
        perm_manager: Arc<PermissionManager>,
    ) -> Self {
        Self {
            _llm: llm,
            speaker,
            explorer,
            thinker,
            supervisor,
            perm_manager,
        }
    }

    pub async fn process_farewell(&self) -> String {
        self.speaker.generate_response("bye").await
    }

    pub async fn process_input(&mut self, text: &str) -> (String, bool) {
        let intent = match self.thinker.determine_intent(text).await {
            Ok(i) => i,
            Err(e) => {
                eprintln!("Intent routing error: {}", e);
                Intent::Chat
            }
        };

        println!("Intent: {:?}", intent);

        let mut web_recap = String::new();
        let mut exit_conversation = false;

        match intent {
            Intent::Chat => {}
            Intent::SearchRequired { query } => match self.explorer.fetch_info(&query).await {
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

                if !self.thinker.ask_yes_no(&ask_prompt).await {
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

                    let search_query = self
                        .thinker
                        .generate_search_query(text, &context_prompt)
                        .await;

                    if !search_query.is_empty() {
                        match self.explorer.fetch_info(&search_query).await {
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
            Intent::ExecuteCommand { command } => match self.perm_manager.execute(&command) {
                Ok(_) => {
                    self.supervisor
                        .store_system_message(format!("Successfully executed command: {}", command))
                        .await;
                }
                Err(e) => {
                    self.supervisor
                        .store_system_message(format!(
                            "Failed to execute command '{}': {}",
                            command, e
                        ))
                        .await;
                }
            },
            Intent::StoreMemory { content, keywords } => {
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

        let current_keywords = self.supervisor.extract_keywords(text).await;
        let all_last_entries_relevant = self.supervisor.evaluate_relevance(text).await;

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

        let final_response = self.speaker.generate_response(&prompt).await;

        (final_response, exit_conversation)
    }
}
