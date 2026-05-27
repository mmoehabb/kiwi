#[async_trait::async_trait]
pub trait WakeWordEngine {
    async fn wait_for_wake_word(&self) -> Result<(), String>;
}

#[async_trait::async_trait]
pub trait SpeechToText {
    async fn listen_and_transcribe(&self) -> Result<String, String>;
}

#[async_trait::async_trait]
pub trait TextToSpeech {
    async fn speak(&self, text: &str) -> Result<Vec<f32>, String>;
}
