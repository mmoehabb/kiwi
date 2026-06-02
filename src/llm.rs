use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait LlmEngine {
    async fn load_model(&mut self, model_path: &str, tokenizer_path: &str) -> Result<(), String>;
    async fn generate(&self, prompt: &str) -> Result<String, String>;
    async fn generate_structured(&self, prompt: &str) -> Result<String, String>;
}

pub struct LocalLlm {
    model_name: String,
    client: reqwest::Client,
}

impl Default for LocalLlm {
    fn default() -> Self {
        Self::new()
    }
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
    pub fn new() -> Self {
        Self {
            model_name: "qwen2.5:1.5b".to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_model(model_name: &str) -> Self {
        Self {
            model_name: model_name.to_string(),
            client: reqwest::Client::new(),
        }
    }

    async fn internal_generate(&self, prompt: &str, is_structured: bool) -> Result<String, String> {
        let url = "http://localhost:11434/api/generate";

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
}

impl Clone for LocalLlm {
    fn clone(&self) -> Self {
        Self {
            model_name: self.model_name.clone(),
            client: self.client.clone(),
        }
    }
}
