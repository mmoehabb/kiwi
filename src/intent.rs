use async_trait::async_trait;
use serde::Deserialize;

use crate::llm::LlmEngine;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Intent {
    Chat,
    SearchRequired { query: String },
    ExecuteCommand { command: String },
}

#[async_trait]
pub trait IntentRouter {
    async fn route_intent(&self, transcribed_text: &str) -> Result<Intent, String>;
}

pub struct LlmIntentRouter<'a> {
    llm: &'a (dyn LlmEngine + Send + Sync),
}

impl<'a> LlmIntentRouter<'a> {
    pub fn new(llm: &'a (dyn LlmEngine + Send + Sync)) -> Self {
        Self { llm }
    }
}

#[async_trait::async_trait]
impl<'a> IntentRouter for LlmIntentRouter<'a> {
    async fn route_intent(&self, transcribed_text: &str) -> Result<Intent, String> {
        let prompt = format!(
            "Analyze the following user input and determine the user's intent. \
            Output ONLY valid JSON. Do not include any markdown formatting or extra text.\n\n\
            Possible intents:\n\
            1. Chat: Normal conversation.\n\
            2. SearchRequired: The user is asking for current events, real-time information, or facts that require searching the web. Include a 'query' field with the search terms.\n\
            3. ExecuteCommand: The user is asking to run a system command or plugin. Include a 'command' field with the command to run.\n\n\
            Examples:\n\
            Input: \"Hello!\"\n\
            Output: {{\"type\": \"Chat\"}}\n\
            Input: \"What is the weather in Tokyo?\"\n\
            Output: {{\"type\": \"SearchRequired\", \"query\": \"weather in Tokyo\"}}\n\
            Input: \"Open the calculator\"\n\
            Output: {{\"type\": \"ExecuteCommand\", \"command\": \"open calculator\"}}\n\n\
            User Input: \"{}\"\n\
            Output:",
            transcribed_text
        );

        let json_response = self.llm.generate_structured(&prompt).await?;

        match serde_json::from_str::<Intent>(&json_response) {
            Ok(intent) => Ok(intent),
            Err(_) => {
                // Fallback to basic heuristics if LLM fails to output valid JSON
                let lower_text = transcribed_text.to_lowercase();
                if lower_text.contains("search")
                    || lower_text.contains("weather")
                    || lower_text.contains("current")
                {
                    Ok(Intent::SearchRequired {
                        query: transcribed_text.to_string(),
                    })
                } else if lower_text.contains("open")
                    || lower_text.contains("run")
                    || lower_text.contains("execute")
                {
                    Ok(Intent::ExecuteCommand {
                        command: transcribed_text.to_string(),
                    })
                } else {
                    Ok(Intent::Chat)
                }
            }
        }
    }
}
