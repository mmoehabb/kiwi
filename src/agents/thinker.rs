use crate::intent::Intent;
use crate::llm::{LlmEngine, LocalLlm};
use std::sync::Arc;

pub struct Thinker {
    llm: Arc<LocalLlm>,
}

impl Thinker {
    pub fn new(llm: Arc<LocalLlm>) -> Self {
        Self { llm }
    }

    pub async fn determine_intent(&self, transcribed_text: &str) -> Result<Intent, String> {
        let prompt = format!(
            "Analyze the following user input and determine the user's intent. \
            Output ONLY valid JSON. Do not include any markdown formatting or extra text.\n\n\
            Possible intents:\n\
            1. Chat: Normal conversation.\n\
            2. SearchRequired: The user is asking for current events, real-time information, or facts that require searching the web. Include a 'query' field with the search terms.\n\
            3. Inquiry: The user is asking a general question where you might already know the answer, but might need to verify if your knowledge is up to date.\n\
            4. ExecuteCommand: The user is asking to run a system command or plugin. Include a 'command' field with the command to run.\n\
            5. StoreMemory: The user is explicitly asking to remember or store something. Include a 'content' field with the raw information to store, and a 'keywords' field containing at least 3 relevant comma-separated keywords for future retrieval.\n\
            6. Farewell: The user is saying goodbye, exiting, or ending the conversation.\n\n\
            Examples:\n\
            Input: \"Hello!\"\n\
            Output: {{\"type\": \"Chat\"}}\n\
            Input: \"What is the weather in Tokyo?\"\n\
            Output: {{\"type\": \"SearchRequired\", \"query\": \"weather in Tokyo\"}}\n\
            Input: \"What is the capital of France?\"\n\
            Output: {{\"type\": \"Inquiry\"}}\n\
            Input: \"Open the calculator\"\n\
            Output: {{\"type\": \"ExecuteCommand\", \"command\": \"open calculator\"}}\n\
            Input: \"Remember that my favorite color is blue\"\n\
            Output: {{\"type\": \"StoreMemory\", \"content\": \"User's favorite color is blue\", \"keywords\": \"favorite, color, blue\"}}\n\
            Input: \"Goodbye!\"\n\
            Output: {{\"type\": \"Farewell\"}}\n\n\
            User Input: \"{}\"\n\
            Output:",
            transcribed_text
        );

        let json_response = match self.llm.generate_structured(&prompt).await {
            Ok(res) => res,
            Err(_) => "".to_string(),
        };

        match serde_json::from_str::<Intent>(&json_response) {
            Ok(intent) => Ok(intent),
            Err(_) => {
                let lower_text = transcribed_text.to_lowercase();
                if lower_text.contains("search")
                    || lower_text.contains("weather")
                    || lower_text.contains("current")
                {
                    Ok(Intent::SearchRequired {
                        query: transcribed_text.to_string(),
                    })
                } else if lower_text.contains("what")
                    || lower_text.contains("how")
                    || lower_text.contains("who")
                    || lower_text.contains("where")
                {
                    Ok(Intent::Inquiry)
                } else if lower_text.contains("open")
                    || lower_text.contains("run")
                    || lower_text.contains("execute")
                {
                    Ok(Intent::ExecuteCommand {
                        command: transcribed_text.to_string(),
                    })
                } else if lower_text.contains("remember") || lower_text.contains("store") {
                    Ok(Intent::StoreMemory {
                        content: transcribed_text.to_string(),
                        keywords: "remember, user".to_string(),
                    })
                } else if lower_text.contains("bye")
                    || lower_text.contains("goodbye")
                    || lower_text.contains("exit")
                    || lower_text.contains("quit")
                {
                    Ok(Intent::Farewell)
                } else {
                    Ok(Intent::Chat)
                }
            }
        }
    }

    pub async fn ask_yes_no(&self, question: &str) -> bool {
        let response = self
            .llm
            .generate(question)
            .await
            .unwrap_or_default()
            .trim()
            .to_lowercase();
        response.contains("yes")
    }

    pub async fn generate_search_query(&self, text: &str) -> String {
        let query_prompt = format!(
            "Generate a short search query to find information about: '{}'. Output ONLY the query.",
            text
        );
        self.llm
            .generate(&query_prompt)
            .await
            .unwrap_or_default()
            .trim()
            .to_string()
    }
}
