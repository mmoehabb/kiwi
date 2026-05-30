/// The Audio component consolidates processing of incoming and outgoing sound.
/// It is responsible for continuous Wake Word detection, Speech-to-Text (STT),
/// and Text-to-Speech (TTS) using the signature parrot persona.

#[async_trait::async_trait]
pub trait WakeWordEngine {
    /// Starts continuously listening to the default microphone.
    /// Blocks or yields until the wake word ("Hey Kiwi") is detected.
    /// TODO: Use a lightweight local library (e.g., rustpotter) for detection.
    async fn wait_for_wake_word(&self) -> Result<(), String>;
}

#[async_trait::async_trait]
pub trait SpeechToText {
    /// Captures a segment of audio and transcribes it into text.
    /// TODO: Implement VAD (Voice Activity Detection) to know when the user stops speaking,
    /// then run a small local Whisper model.
    async fn listen_and_transcribe(&self) -> Result<String, String>;
}

#[async_trait::async_trait]
pub trait TextToSpeech {
    /// Converts text into audio using the "parrot" persona and plays it.
    /// TODO: Integrate a fast local TTS engine (e.g., Piper) and stream to the audio output.
    async fn speak(&self, text: &str) -> Result<(), String>;
}

/// The unified manager for all audio operations.
pub struct AudioManager {
    // TODO: Add fields for audio streams, input/output device handles, and STT/TTS models.
}

impl AudioManager {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl WakeWordEngine for AudioManager {
    async fn wait_for_wake_word(&self) -> Result<(), String> {
        // TODO: Open mic stream, process frames, return when "Hey Kiwi" is heard.
        Ok(())
    }
}

#[async_trait::async_trait]
impl SpeechToText for AudioManager {
    async fn listen_and_transcribe(&self) -> Result<String, String> {
        // TODO: Record until silence, pass buffer to whisper model, return string.
        Ok("This is a transcribed sentence.".to_string())
    }
}

#[async_trait::async_trait]
impl TextToSpeech for AudioManager {
    async fn speak(&self, _text: &str) -> Result<(), String> {
        // TODO: Synthesize speech from text, output through speakers with parrot filters.
        Ok(())
    }
}
