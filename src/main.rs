use kiwi::agents::{Explorer, Orchestrator, Speaker, Supervisor, Thinker};
use kiwi::audio::AudioManager;
use kiwi::config::Configuration;
use kiwi::daemon::run_background_daemon;
use kiwi::event::KiwiEvent;
use kiwi::gui::{KiwiGui, MascotState};
use kiwi::llm::{LlmEngine, LocalLlm};
use kiwi::memory::MemoryBank;
use kiwi::onboarding::run_onboarding;
use kiwi::permissions::PermissionManager;
use kiwi::wakeword::WakewordEngine;
use kiwi::web::{WebClient, WebTool};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Kiwi...");

    let config = Arc::new(Configuration::new());
    let wakeword_path = Configuration::wakeword_templates_path().unwrap();
    let wakeword_engine = WakewordEngine::new(wakeword_path, config.app.wake_word_sensitivity);
    let wakeword_engine_arc = Arc::new(Mutex::new(wakeword_engine));

    println!("Initializing LLM Engine with Ollama...");
    let mut _llm = LocalLlm::new(config.clone());
    _llm.load_model("", "").await?;

    let audio_mgr = Arc::new(AudioManager::new(config.clone()).await?);

    let (event_tx, mut event_rx) = mpsc::channel::<KiwiEvent>(32);
    let (gui_tx, gui_rx) = mpsc::channel::<MascotState>(32);
    let audio_mgr_clone = audio_mgr.clone();

    let memory_bank = MemoryBank::new(2048)
        .await
        .expect("Failed to initialize memory bank");

    let perm_manager = Arc::new(PermissionManager::load().unwrap_or_else(|_| {
        kiwi::permissions::PermissionManager::from_file(std::path::Path::new("/dev/null"))
            .unwrap_or_else(|_| {
                let mut p = std::env::temp_dir();
                p.push("kiwi_empty_perms.toml");
                std::fs::write(&p, "").unwrap_or_default();
                kiwi::permissions::PermissionManager::from_file(&p).unwrap()
            })
    }));

    // Initialize Agents
    let speaker_llm = Arc::new(
        LocalLlm::with_model(config.clone(), config.app.speaker_model.clone())
            .with_agent_role(true, false),
    );
    let explorer_llm = Arc::new(
        LocalLlm::with_model(config.clone(), config.app.explorer_model.clone())
            .with_agent_role(false, false),
    );
    let thinker_llm = Arc::new(
        LocalLlm::with_model(config.clone(), config.app.thinker_model.clone())
            .with_agent_role(false, true),
    );
    let supervisor_llm = Arc::new(
        LocalLlm::with_model(config.clone(), config.app.supervisor_model.clone())
            .with_agent_role(false, false),
    );
    let orchestrator_llm = Arc::new(
        LocalLlm::with_model(config.clone(), config.app.orchestrator_model.clone())
            .with_agent_role(false, false),
    );

    let speaker = Speaker::new(speaker_llm);

    let web_client = Arc::new(WebClient::new(config.clone()));
    let web_tool = Arc::new(WebTool::new(web_client.clone()));

    let explorer = Explorer::new(explorer_llm, web_tool);

    let thinker = Thinker::new(thinker_llm);

    let supervisor = Supervisor::new(supervisor_llm, memory_bank);

    let orchestrator = Orchestrator::new(
        orchestrator_llm,
        speaker,
        explorer,
        thinker,
        supervisor,
        perm_manager.clone(),
    );

    let (gui_event_tx, gui_event_rx) = mpsc::channel(10);
    let gui_event_tx_clone = gui_event_tx.clone();
    let wakeword_engine_arc_clone = wakeword_engine_arc.clone();
    let gui_tx_clone = gui_tx.clone();
    let config_daemon = config.clone();

    tokio::spawn(async move {
        let has_templates = {
            let engine = wakeword_engine_arc_clone.lock().await;
            engine.has_templates()
        };

        if !has_templates {
            run_onboarding(
                gui_tx_clone,
                gui_event_rx,
                wakeword_engine_arc_clone.clone(),
            )
            .await;
        }

        run_background_daemon(
            audio_mgr_clone,
            wakeword_engine_arc_clone,
            config_daemon,
            event_tx,
            orchestrator,
        )
        .await;
    });

    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            match event {
                KiwiEvent::WakeWordDetected => {
                    let _ = gui_tx.send(MascotState::Listening).await;
                }
                KiwiEvent::TranscribedText(_) => {
                    let _ = gui_tx.send(MascotState::Thinking).await;
                }
                KiwiEvent::AssistantResponse(_) => {
                    let _ = gui_tx.send(MascotState::Speaking).await;
                }
                KiwiEvent::Idle => {
                    let _ = gui_tx.send(MascotState::Idle).await;
                }
            }
        }
    });

    let options = eframe::NativeOptions {
        ..Default::default()
    };

    eframe::run_native(
        "Kiwi",
        options,
        Box::new(move |_cc| Ok(Box::new(KiwiGui::new(gui_rx, gui_event_tx_clone)))),
    )?;

    Ok(())
}
