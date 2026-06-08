import sys

def modify():
    with open("src/audio.rs", "r") as f:
        content = f.read()

    # The issue is we injected `impl WakeWordListener` into the middle of nothing or overrode the trait definition
    # Let's fix the file directly since we know the exact issue.
    # The error says "}impl WakeWordListener for AudioManager {"
    content = content.replace("}impl WakeWordListener for AudioManager {", "\nimpl WakeWordListener for AudioManager {")
    content = content.replace("use std::sync::Arc;", "use std::sync::Arc;\nuse crate::wakeword::WakewordEngine;")
    # Trait WakeWordListener might be missing if we replaced it.
    if "trait WakeWordListener" not in content:
        content = """#[async_trait::async_trait]
pub trait WakeWordListener {
    async fn wait_for_wake_word(&self, engine: Arc<tokio::sync::Mutex<WakewordEngine>>) -> Result<(), String>;
}
""" + content

    with open("src/audio.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify()
