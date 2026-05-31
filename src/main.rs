pub mod audio;
pub mod config;
pub mod gui;
pub mod llm;
pub mod memory;
pub mod plugin;
pub mod web;

use crate::gui::KiwiGui;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🦜 Starting Kiwi...");

    // 1. Load Configuration & Permissions
    // let mut config_mgr = config::Configuration::new();
    // config_mgr.load_from_file("~/.config/kiwi/permissions.toml").ok();

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

    // Create a channel for UI state updates
    let (_tx, rx) = mpsc::channel(100);

    // Spawn the background pipeline task
    tokio::spawn(async move {
        // TODO: integrate background events properly via the rx channel.
        // For example:
        // loop {
        //     // audio_mgr.wait_for_wake_word().await;
        //     let _ = tx.send(MascotState::Listening).await;
        //     // let text = audio_mgr.listen_and_transcribe().await;
        //     // let intent = router.route_intent(&text).await;
        //     let _ = tx.send(MascotState::Thinking).await;
        //     // ... route to LLM, Web, or Plugin ...
        //     // let response = llm_engine.generate(...).await;
        //     let _ = tx.send(MascotState::Speaking).await;
        //     // audio_mgr.speak(&response).await;
        //     let _ = tx.send(MascotState::Idle).await;
        // }
    });

    let options = eframe::NativeOptions {
        // Configure transparent, un-decorated window for the mascot.
        viewport: eframe::egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_inner_size([320.0, 320.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Kiwi 🦜",
        options,
        Box::new(|_cc| Ok(Box::new(KiwiGui::new(rx)))),
    )?;

    Ok(())
}
