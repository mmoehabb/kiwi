use crate::config::Configuration;
use async_trait::async_trait;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{LlamaChatMessage, LlamaChatTemplate, LlamaModel};
use llama_cpp_2::token::data_array::LlamaTokenDataArray;
use serde::Deserialize;
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

lazy_static::lazy_static! {
    static ref LLAMA_BACKEND: LlamaBackend = LlamaBackend::init().expect("Failed to initialize llama-cpp backend");
    static ref LOADED_MODELS: Arc<Mutex<HashMap<String, Arc<LlamaModel>>>> = Arc::new(Mutex::new(HashMap::new()));
}

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
    is_speaker: bool,
    is_thinker: bool,
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
            is_speaker: false,
            is_thinker: false,
        }
    }

    pub fn with_agent_role(mut self, is_speaker: bool, is_thinker: bool) -> Self {
        self.is_speaker = is_speaker;
        self.is_thinker = is_thinker;
        self
    }

    async fn download_model_if_missing(&self) -> Result<PathBuf, String> {
        let models_dir = Configuration::models_dir()?;
        let model_path = models_dir.join(format!("{}.gguf", self.model_name));

        if !model_path.exists() {
            println!(
                "Downloading model {} from {}...",
                self.model_name, self.config.app.llm_model_url
            );
            let url = &self.config.app.llm_model_url;

            let response = self
                .client
                .get(url)
                .send()
                .await
                .map_err(|e| format!("Request failed: {}", e))?;

            if !response.status().is_success() {
                return Err(format!(
                    "Failed to download model: HTTP {}",
                    response.status()
                ));
            }

            let mut file = tokio::fs::File::create(&model_path)
                .await
                .map_err(|e| format!("Failed to create model file: {}", e))?;
            use futures_util::StreamExt;
            let mut stream = response.bytes_stream();
            while let Some(item) = stream.next().await {
                let chunk = item.map_err(|e| format!("Error while downloading: {}", e))?;
                tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
                    .await
                    .map_err(|e| format!("Error writing to file: {}", e))?;
            }
            println!("Download complete: {:?}", model_path);
        }

        Ok(model_path)
    }

    async fn get_or_load_model(&self) -> Result<Arc<LlamaModel>, String> {
        let mut models: tokio::sync::MutexGuard<'_, HashMap<String, Arc<LlamaModel>>> =
            LOADED_MODELS.lock().await;

        if let Some(model) = models.get(&self.model_name) {
            return Ok(model.clone());
        }

        let model_path = self.download_model_if_missing().await?;

        println!("Loading model into memory: {:?}", model_path);

        let model_arc = tokio::task::spawn_blocking(move || {
            let model_params = LlamaModelParams::default();
            let backend = &LLAMA_BACKEND;
            let model = LlamaModel::load_from_file(backend, &model_path, &model_params)
                .map_err(|e| format!("Failed to load model: {}", e))?;
            Ok::<Arc<LlamaModel>, String>(Arc::new(model))
        })
        .await
        .map_err(|e| format!("Failed to spawn blocking load task: {}", e))??;

        models.insert(self.model_name.clone(), model_arc.clone());

        Ok(model_arc)
    }

    async fn internal_generate(&self, prompt: &str, is_structured: bool) -> Result<String, String> {
        let model = self.get_or_load_model().await?;

        let mut chat_messages = Vec::new();
        if self.is_speaker {
            chat_messages.push(
                LlamaChatMessage::new("system".to_string(), self.config.app.system_message.clone())
                    .map_err(|e| format!("Failed to create system message: {}", e))?,
            );
        } else if is_structured {
            chat_messages.push(
                LlamaChatMessage::new(
                    "system".to_string(),
                    "You must output only valid JSON.".to_string(),
                )
                .map_err(|e| format!("Failed to create system message: {}", e))?,
            );
        }
        chat_messages.push(
            LlamaChatMessage::new("user".to_string(), prompt.to_string())
                .map_err(|e| format!("Failed to create user message: {}", e))?,
        );

        let formatted_prompt = tokio::task::spawn_blocking({
            let model_clone = model.clone();
            move || {
                let builtin_tmpl = model_clone
                    .chat_template(None)
                    .unwrap_or_else(|_| LlamaChatTemplate::new("chatml").unwrap());
                model_clone
                    .apply_chat_template(&builtin_tmpl, &chat_messages, true)
                    .map_err(|e| format!("Failed to apply chat template: {:?}", e))
            }
        })
        .await
        .map_err(|e| format!("Failed to spawn template task: {}", e))??;

        let num_ctx = self.config.app.num_ctx;

        let result = tokio::task::spawn_blocking(move || {
            let mut ctx_params = LlamaContextParams::default();
            ctx_params = ctx_params.with_n_ctx(NonZeroU32::new(num_ctx));

            let mut ctx = model
                .new_context(&LLAMA_BACKEND, ctx_params)
                .map_err(|e| format!("Failed to create context: {}", e))?;

            let tokens = model
                .str_to_token(&formatted_prompt, llama_cpp_2::model::AddBos::Always)
                .map_err(|e| format!("Failed to tokenize prompt: {}", e))?;

            let mut batch = LlamaBatch::new(num_ctx as usize, 1);
            let last_index = tokens.len() - 1;
            for (i, token) in tokens.into_iter().enumerate() {
                batch
                    .add(token, i as i32, &[0], i == last_index)
                    .map_err(|e| format!("Failed to add to batch: {}", e))?;
            }

            ctx.decode(&mut batch)
                .map_err(|e| format!("Failed to decode: {}", e))?;

            let mut response = String::new();
            let mut n_cur = batch.n_tokens();
            let n_predict = 1024; // Arbitrary max tokens
            let mut decoder = encoding_rs::UTF_8.new_decoder();

            while n_cur <= n_predict {
                let candidates = ctx.candidates_ith(batch.n_tokens() - 1);
                let mut candidates_p = LlamaTokenDataArray::from_iter(candidates, false);

                let new_token = candidates_p.sample_token_greedy();
                let new_token_id = new_token;

                if new_token_id == model.token_eos() {
                    break;
                }

                let new_token_str = model
                    .token_to_piece(new_token_id, &mut decoder, false, None)
                    .map_err(|e| format!("Failed to convert token to piece: {}", e))?;

                response.push_str(&new_token_str);

                batch.clear();
                batch
                    .add(new_token_id, n_cur, &[0], true)
                    .map_err(|e| format!("Failed to add new token to batch: {}", e))?;

                ctx.decode(&mut batch)
                    .map_err(|e| format!("Failed to decode next step: {}", e))?;
                n_cur += 1;
            }

            Ok::<String, String>(response.trim().to_string())
        })
        .await
        .map_err(|e| format!("Task failed: {}", e))??;

        Ok(result)
    }
}

#[async_trait::async_trait]
impl LlmEngine for LocalLlm {
    async fn load_model(&mut self, _model_path: &str, _tokenizer_path: &str) -> Result<(), String> {
        // Will be lazily loaded in internal_generate or we can trigger it here
        self.get_or_load_model().await?;
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
            is_speaker: self.is_speaker,
            is_thinker: self.is_thinker,
        }
    }
}
