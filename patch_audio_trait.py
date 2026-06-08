import sys

def modify():
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
    if old_trait in content:
        content = content.replace(old_trait, new_trait)
    else:
        print("OLD TRAIT NOT FOUND EXACTLY")

    with open("src/audio.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify()
