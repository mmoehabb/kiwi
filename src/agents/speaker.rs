use crate::llm::{LlmEngine, LocalLlm};
use std::sync::Arc;

pub struct Speaker {
    llm: Arc<LocalLlm>,
}

impl Speaker {
    pub fn new(llm: Arc<LocalLlm>) -> Self {
        Self { llm }
    }

    pub async fn generate_response(&self, prompt: &str) -> String {
        match self.llm.generate(prompt).await {
            Ok(res) => res,
            Err(e) => format!("Error generating response: {}", e),
        }
    }
}
