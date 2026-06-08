import sys

def modify():
    with open("src/audio.rs", "r") as f:
        content = f.read()

    # The trait is called WakeWordEngine in audio.rs, but we implement WakeWordListener later.
    # Ah! The original code says `pub trait WakeWordListener` but in `src/audio.rs` head it says `pub trait WakeWordEngine`.
    # Let's fix that.

    old_trait = """#[async_trait]
pub trait WakeWordEngine {
    async fn wait_for_wake_word(&self) -> Result<(), String>;
}"""
    new_trait = """#[async_trait::async_trait]
pub trait WakeWordListener {
    async fn wait_for_wake_word(&self, engine: std::sync::Arc<tokio::sync::Mutex<crate::wakeword::WakewordEngine>>) -> Result<(), String>;
}"""
    content = content.replace(old_trait, new_trait)

    with open("src/audio.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify()
