use crate::config::Configuration;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[async_trait]
pub trait LlmEngine {
    async fn load_model(&mut self, model_path: &str, tokenizer_path: &str) -> Result<(), String>;
    async fn generate(&self, prompt: &str) -> Result<String, String>;
    async fn generate_structured(&self, prompt: &str) -> Result<String, String>;
    async fn extract_keywords(&self, text: &str) -> Result<Vec<String>, String>;
}

pub struct LocalLlm {
    model_name: String,
    client: reqwest::Client,
    config: Arc<Configuration>,
}

#[derive(Serialize)]
struct OllamaGenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<&'a str>,
}

#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
}

impl LocalLlm {
    pub fn new(config: Arc<Configuration>) -> Self {
        Self::with_model(config.clone(), config.app.model_name.clone())
    }

    pub fn with_model(config: Arc<Configuration>, model_name: String) -> Self {
        Self {
            model_name,
            client: reqwest::Client::new(),
            config,
        }
    }

    async fn internal_generate(&self, prompt: &str, is_structured: bool) -> Result<String, String> {
        let url = &self.config.app.llm_url;

        let format = if is_structured { Some("json") } else { None };

        let request = OllamaGenerateRequest {
            model: &self.model_name,
            prompt,
            stream: false,
            format,
        };

        let res = self
            .client
            .post(url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Failed to connect to Ollama: {}", e))?;

        if !res.status().is_success() {
            return Err(format!("Ollama API error: {}", res.status()));
        }

        let ollama_res: OllamaGenerateResponse = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;

        Ok(ollama_res.response)
    }
}

#[async_trait::async_trait]
impl LlmEngine for LocalLlm {
    async fn load_model(&mut self, _model_path: &str, _tokenizer_path: &str) -> Result<(), String> {
        // Ollama manages models internally. We can just pull if we want, but let's assume it's there.
        // Or we could trigger a `pull` API request here, but that might block for a long time.
        // For simplicity, we assume the user has pulled the model, or Ollama handles it.
        Ok(())
    }

    async fn generate(&self, prompt: &str) -> Result<String, String> {
        self.internal_generate(prompt, false).await
    }

    async fn generate_structured(&self, prompt: &str) -> Result<String, String> {
        self.internal_generate(prompt, true).await
    }

    async fn extract_keywords(&self, text: &str) -> Result<Vec<String>, String> {
        let prompt = format!(
            "Analyze the following text and extract at least 3 relevant keywords. \
            Output ONLY valid JSON. Do not include any markdown formatting or extra text.\n\n\
            Format: {{\"keywords\": \"keyword1, keyword2, keyword3\"}}\n\n\
            Text: \"{}\"\n\
            Output:",
            text
        );

        let json_response = self.generate_structured(&prompt).await?;

        #[derive(Deserialize)]
        struct KeywordResponse {
            keywords: String,
        }

        match serde_json::from_str::<KeywordResponse>(&json_response) {
            Ok(resp) => {
                let keywords: Vec<String> = resp
                    .keywords
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
                Ok(keywords)
            }
            Err(_) => {
                // Fallback to basic word splitting if LLM fails
                let words: Vec<String> = text
                    .split_whitespace()
                    .take(3)
                    .map(|s| s.to_string())
                    .collect();
                Ok(words)
            }
        }
    }
}

impl Clone for LocalLlm {
    fn clone(&self) -> Self {
        Self {
            model_name: self.model_name.clone(),
            client: self.client.clone(),
            config: self.config.clone(),
        }
    }
}
