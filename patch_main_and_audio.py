import sys

def modify():
    with open("src/main.rs", "r") as f:
        content = f.read()

    # Import fix: crate name is `kiwi` not `kiwi_core`
    content = content.replace("use kiwi_core::wakeword::WakewordEngine;", "use kiwi::wakeword::WakewordEngine;")

    # Type hints
    content = content.replace("let engine = wakeword_engine_arc_clone.lock().await;", "let engine: tokio::sync::MutexGuard<kiwi::wakeword::WakewordEngine> = wakeword_engine_arc_clone.lock().await;")
    content = content.replace("let mut engine = wakeword_engine_arc_clone.lock().await;", "let mut engine: tokio::sync::MutexGuard<kiwi::wakeword::WakewordEngine> = wakeword_engine_arc_clone.lock().await;")

    with open("src/main.rs", "w") as f:
        f.write(content)

    with open("src/audio.rs", "r") as f:
        content = f.read()

    old_trait = """#[async_trait::async_trait]
pub trait WakeWordListener {
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
