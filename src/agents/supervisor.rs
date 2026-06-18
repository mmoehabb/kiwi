use crate::llm::{LlmEngine, LocalLlm};
use crate::memory::{ContextManager, MemoryBank, Message};
use std::sync::Arc;

pub struct Supervisor {
    llm: Arc<LocalLlm>,
    pub memory_bank: MemoryBank,
}

impl Supervisor {
    pub fn new(llm: Arc<LocalLlm>, memory_bank: MemoryBank) -> Self {
        Self { llm, memory_bank }
    }

    pub async fn store_memory(&mut self, content: String, keywords: String) {
        let _ = self
            .memory_bank
            .add_message(Message {
                role: "user".to_string(),
                content,
                keywords: Some(keywords),
            })
            .await;
    }

    pub async fn store_system_message(&mut self, content: String) {
        let _ = self
            .memory_bank
            .add_message(Message {
                role: "system".to_string(),
                content,
                keywords: None,
            })
            .await;
    }

    pub async fn extract_keywords(&self, text: &str) -> Vec<String> {
        self.llm.extract_keywords(text).await.unwrap_or_default()
    }

    pub async fn evaluate_relevance(&self, text: &str) -> Vec<bool> {
        let mut results = Vec::new();
        let history_len = self.memory_bank.history.len();
        let last_five_start = history_len.saturating_sub(5);

        for (i, msg) in self.memory_bank.history.iter().enumerate() {
            if i < last_five_start {
                continue;
            }
            if msg.role == "system" {
                results.push(false);
                continue;
            }

            let prompt = format!(
                "Are the following two texts relevant to the same topic? \
                Reply ONLY with 'Yes' or 'No'.\n\n\
                Text 1: \"{}\"\n\
                Text 2: \"{}\"\n\n\
                Output:",
                msg.content, text
            );

            let is_relevant = self
                .llm
                .generate(&prompt)
                .await
                .unwrap_or_default()
                .trim()
                .to_lowercase()
                .contains("yes");

            results.push(is_relevant);
        }
        results
    }
}
