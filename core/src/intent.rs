use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Intent {
    Chat,
    SearchRequired { query: String },
    ExecuteCommand { command: String },
}

#[async_trait::async_trait]
pub trait IntentRouter {
    async fn route_intent(&self, transcribed_text: &str) -> Result<Intent, String>;
}
