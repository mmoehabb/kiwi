#[derive(Debug)]
pub enum KiwiEvent {
    WakeWordDetected,
    TranscribedText(String),
    AssistantResponse(String),
}
