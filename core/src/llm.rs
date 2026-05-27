#[async_trait::async_trait]
pub trait LlmEngine: Send + Sync {
    async fn load_model(&mut self, model_path: &str, tokenizer_path: &str) -> Result<(), String>;
    async fn generate(&self, prompt: &str) -> Result<String, String>;
    async fn generate_structured(&self, prompt: &str) -> Result<String, String>;
}
