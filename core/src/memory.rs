use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[async_trait::async_trait]
pub trait ContextManager {
    async fn add_message(&mut self, message: Message) -> Result<(), String>;
    fn build_prompt(&self) -> String;
    async fn clear(&mut self) -> Result<(), String>;
}
