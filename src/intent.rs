use kiwi_core::intent::{Intent, IntentRouter};

use kiwi_core::llm::LlmEngine;

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

#[cfg(test)]
mod tests {
    use super::*;

    struct MockLlm;
    #[async_trait::async_trait]
    impl kiwi_core::llm::LlmEngine for MockLlm {
        async fn load_model(&mut self, _m: &str, _t: &str) -> Result<(), String> {
            Ok(())
        }
        async fn generate(&self, _p: &str) -> Result<String, String> {
            Ok("".to_string())
        }
        async fn generate_structured(&self, _p: &str) -> Result<String, String> {
            Ok(r#"{"type": "SearchRequired", "query": "test query"}"#.to_string())
        }
    }

    #[tokio::test]
    async fn test_intent_routing() {
        let llm = MockLlm;
        let router = LlmIntentRouter::new(&llm);

        let intent = router.route_intent("search something").await.unwrap();
        match intent {
            Intent::SearchRequired { query } => assert_eq!(query, "test query"),
            _ => panic!("Expected SearchRequired intent"),
        }
    }
}
