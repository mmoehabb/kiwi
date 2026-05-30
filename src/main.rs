pub mod audio;
pub mod config;
pub mod executor;
pub mod gui;
pub mod llm;
pub mod memory;
pub mod plugin;
pub mod web;

use crate::gui::KiwiGui;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🦜 Starting Kiwi...");

    // 1. Load Configuration & Permissions
    let mut config_mgr = config::Configuration::new();
    if let Err(e) = config_mgr.load_from_file("~/.config/kiwi/permissions.toml") {
        eprintln!("Warning: Failed to load config: {}", e);
    }

    // 2. Initialize LLM
    // let mut llm_engine = llm::LocalLlm::new();
    // llm_engine.load_model("models/kiwi-7b.gguf").await.ok();

    // 3. Initialize Plugins
    // let mut plugin_mgr = plugin::RhaiEngine::new();

    // 4. Initialize Audio (Wake word, STT, TTS)
    // let audio_mgr = audio::AudioManager::new();

    // 5. Initialize Memory & Context
    // let mut memory = memory::MemoryBank::new(2048);

    // 6. Initialize Web Search
    // let web_client = web::WebClient::new();

    // 7. Start GUI (This usually takes over the main thread in eframe)
    // For a real application, the background services (Audio, LLM, etc.)
    // would be spawned into separate Tokio tasks communicating via channels,
    // while the GUI runs on the main thread.

    /*
    tokio::spawn(async move {
        loop {
            // Background pipeline conceptual flow:
            // audio_mgr.wait_for_wake_word().await;
            // update_gui_state(Listening);
            // let text = audio_mgr.listen_and_transcribe().await;
            // let intent = router.route_intent(&text).await;
            // ... route to LLM, Web, or Plugin ...
            // let response = llm_engine.generate(...).await;
            // update_gui_state(Speaking);
            // audio_mgr.speak(&response).await;
            // update_gui_state(Idle);
        }
    });
    */

    let options = eframe::NativeOptions {
        // TODO: Configure transparent, un-decorated window for the mascot.
        ..Default::default()
    };

    eframe::run_native(
        "Kiwi 🦜",
        options,
        Box::new(|_cc| Ok(Box::new(KiwiGui::new()))),
    )?;

    Ok(())
}
