#[derive(Debug, Clone)]
pub enum KiwiEvent {
    WakeWordDetected,
    TranscribedText(String),
    AssistantResponse(String),
}
