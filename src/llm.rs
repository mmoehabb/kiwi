use candle_core::quantized::gguf_file;
use candle_core::{Device, Tensor};
use candle_transformers::models::quantized_llama::ModelWeights;
use kiwi_core::llm::LlmEngine;
use std::sync::{Arc, Mutex};
use tokenizers::Tokenizer;

/// The main struct for managing local inference.
pub struct LocalLlm {
    model: Option<Arc<Mutex<ModelWeights>>>,
    tokenizer: Option<Arc<Tokenizer>>,
    device: Device,
    temperature: f64,
    top_p: f64,
    seed: u64,
}

impl Default for LocalLlm {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalLlm {
    pub fn new() -> Self {
        Self {
            model: None,
            tokenizer: None,
            device: Device::Cpu,
            temperature: 0.7,
            top_p: 0.9,
            seed: 299792458,
        }
    }

    // Internal helper for generation logic
    fn internal_generate(&self, prompt: &str, _is_structured: bool) -> Result<String, String> {
        let model_mutex = self.model.as_ref().ok_or("Model not loaded")?;
        let tokenizer = self.tokenizer.as_ref().ok_or("Tokenizer not loaded")?;

        let mut model = model_mutex.lock().map_err(|e| e.to_string())?;

        let tokens = tokenizer
            .encode(prompt, true)
            .map_err(|e| e.to_string())?
            .get_ids()
            .to_vec();
        let prompt_len = tokens.len();

        let mut all_tokens = vec![];
        let mut next_token;
        let eos_token = tokenizer.token_to_id("<|eot_id|>").unwrap_or(2); // Llama 3 eos token fallback

        let mut logits_processor = candle_transformers::generation::LogitsProcessor::new(
            self.seed,
            Some(self.temperature),
            Some(self.top_p),
        );

        let mut input = Tensor::new(tokens.as_slice(), &self.device).map_err(|e| e.to_string())?;
        input = input.unsqueeze(0).map_err(|e| e.to_string())?;

        for index in 0..1024 {
            // max tokens
            let pos = if index == 0 {
                0
            } else {
                prompt_len + index - 1
            };
            let logits = model.forward(&input, pos).map_err(|e| e.to_string())?;
            let logits = logits.squeeze(0).map_err(|e| e.to_string())?;

            // Logits is shape (seq_len, vocab_size). We need the last token's logits.
            let seq_len = logits.dim(0).map_err(|e| e.to_string())?;
            let logits = logits.get(seq_len - 1).map_err(|e| e.to_string())?;

            // If structured, we might want to guide it to output JSON
            // For now, prompt engineering handles it, but we could add grammar sampling here.

            next_token = logits_processor
                .sample(&logits)
                .map_err(|e| e.to_string())?;

            if next_token == eos_token {
                break;
            }
            all_tokens.push(next_token);

            input = Tensor::new(&[next_token], &self.device)
                .map_err(|e| e.to_string())?
                .unsqueeze(0)
                .map_err(|e| e.to_string())?;
        }

        let generated_text = tokenizer
            .decode(&all_tokens, true)
            .map_err(|e| e.to_string())?;
        Ok(generated_text)
    }
}

#[async_trait::async_trait]
impl LlmEngine for LocalLlm {
    async fn load_model(&mut self, model_path: &str, tokenizer_path: &str) -> Result<(), String> {
        let model_path = model_path.to_string();
        let tokenizer_path = tokenizer_path.to_string();
        let device = self.device.clone();

        let (weights, tokenizer) = tokio::task::spawn_blocking(move || {
            let mut file = std::fs::File::open(&model_path).map_err(|e| e.to_string())?;
            let reader = gguf_file::Content::read(&mut file).map_err(|e| e.to_string())?;
            let weights =
                ModelWeights::from_gguf(reader, &mut file, &device).map_err(|e| e.to_string())?;

            let tokenizer = Tokenizer::from_file(&tokenizer_path).map_err(|e| e.to_string())?;

            Ok::<_, String>((weights, tokenizer))
        })
        .await
        .map_err(|e| e.to_string())??;

        self.model = Some(Arc::new(Mutex::new(weights)));
        self.tokenizer = Some(Arc::new(tokenizer));

        Ok(())
    }

    async fn generate(&self, prompt: &str) -> Result<String, String> {
        let prompt_clone = prompt.to_string();
        let this = self.clone();

        tokio::task::spawn_blocking(move || this.internal_generate(&prompt_clone, false))
            .await
            .map_err(|e| e.to_string())?
    }

    async fn generate_structured(&self, prompt: &str) -> Result<String, String> {
        let prompt_clone = prompt.to_string();
        let this = self.clone();

        tokio::task::spawn_blocking(move || this.internal_generate(&prompt_clone, true))
            .await
            .map_err(|e| e.to_string())?
    }
}

impl Clone for LocalLlm {
    fn clone(&self) -> Self {
        Self {
            model: self.model.clone(),
            tokenizer: self.tokenizer.clone(),
            device: self.device.clone(),
            temperature: self.temperature,
            top_p: self.top_p,
            seed: self.seed,
        }
    }
}
