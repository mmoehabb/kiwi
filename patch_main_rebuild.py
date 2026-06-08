import sys

def modify():
    with open("src/main.rs", "r") as f:
        content = f.read()

    new_imports = """use kiwi::audio::{AudioManager, SpeechToText, TextToSpeech, WakeWordListener};
use kiwi::config::Configuration;
use kiwi::event::KiwiEvent;
use kiwi::gui::{KiwiGui, MascotState, GuiEvent};
use kiwi::intent::IntentEngine;
use kiwi::interruption::InterruptionDetector;
use kiwi::llm::{LlmEngine, LocalLlm};
use kiwi::memory::ContextManager;
use kiwi::permissions::CommandExecutor;
use kiwi::plugin::PluginEngine;
use kiwi::web::WebTool;
use rodio::{OutputStream, Sink};
use std::sync::Arc;
use kiwi::wakeword::WakewordEngine;
"""
    idx = content.find("fn main() -> Result<(), String> {")
    content = new_imports + "\n" + content[idx:]

    init_engine = """    let config = Arc::new(Configuration::new());

    let wakeword_path = Configuration::wakeword_templates_path().unwrap();
    let mut wakeword_engine = WakewordEngine::new(wakeword_path, config.app.wake_word_sensitivity);
    let wakeword_engine_arc = Arc::new(tokio::sync::Mutex::new(wakeword_engine));
"""
    content = content.replace("    let config = Arc::new(Configuration::new());", init_engine)

    channel_setup = """
    let (gui_event_tx, mut gui_event_rx) = tokio::sync::mpsc::channel(10);
    let gui_event_tx_clone = gui_event_tx.clone();

    let wakeword_engine_arc_clone = wakeword_engine_arc.clone();
    let gui_tx_clone = gui_tx.clone();

    tokio::spawn(async move {
        let has_templates = {
            let engine = wakeword_engine_arc_clone.lock().await;
            engine.has_templates()
        };

        if !has_templates {
            let _ = gui_tx_clone.send(MascotState::Onboarding).await;
            while let Some(event) = gui_event_rx.recv().await {
                match event {
                    GuiEvent::RecordSample => {
                        println!("Recording sample...");
                        let (audio_data, _rate) = tokio::task::spawn_blocking(|| {
                            use ringbuf::traits::{Producer, Consumer, Split};
                            let host = cpal::default_host();
                            use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
                            let device = host.default_input_device().unwrap();
                            let conf = device.default_input_config().unwrap();
                            let channels = conf.channels();

                            let rb = ringbuf::HeapRb::<f32>::new(16000 * 2);
                            let (mut prod, mut cons) = ringbuf::traits::Split::split(rb);

                            let stream = match conf.sample_format() {
                                cpal::SampleFormat::F32 => device.build_input_stream(
                                    &conf.clone().into(),
                                    move |data: &[f32], _| {
                                        for frame in data.chunks(channels as usize) {
                                            let mono_sample = frame.iter().sum::<f32>() / channels as f32;
                                            let _ = ringbuf::traits::Producer::try_push(&mut prod, mono_sample);
                                        }
                                    },
                                    |err| eprintln!("an error occurred on stream: {}", err),
                                    None,
                                ).unwrap(),
                                _ => panic!("Unsupported format"),
                            };

                            stream.play().unwrap();
                            std::thread::sleep(std::time::Duration::from_millis(2000));
                            stream.pause().unwrap();

                            let mut buf = Vec::new();
                            while let Some(s) = ringbuf::traits::Consumer::try_pop(&mut cons) { buf.push(s); }

                            (buf, conf.sample_rate().0)
                        }).await.unwrap();

                        let processed = if _rate != 16000 {
                            use dasp::{signal, Signal, interpolate::linear::Linear};
                            let mut sig = signal::from_iter(audio_data.clone());
                            let interp = Linear::new(sig.next(), sig.next());
                            sig.from_hz_to_hz(interp, _rate as f64, 16000.0)
                               .take((audio_data.len() as f64 * (16000.0 / _rate as f64)) as usize)
                               .collect()
                        } else {
                            audio_data
                        };

                        let mut engine = wakeword_engine_arc_clone.lock().await;
                        engine.add_template(&processed);
                        println!("Sample recorded!");
                    }
                    GuiEvent::DoneOnboarding => {
                        let mut engine = wakeword_engine_arc_clone.lock().await;
                        let _ = engine.save_templates();
                        println!("Onboarding complete.");
                        let _ = gui_tx_clone.send(MascotState::Idle).await;
                        break;
                    }
                }
            }
        }

        let audio_mgr_clone = audio_mgr.clone();
        println!("Background daemon started. Listening for wake word...");
        loop {
            if let Err(e) = audio_mgr_clone.wait_for_wake_word(wakeword_engine_arc_clone.clone()).await {"""

    idx1 = content.find("    tokio::spawn(async move {")
    idx2 = content.find("if let Err(e) = audio_mgr_clone.wait_for_wake_word().await {")

    if idx1 != -1 and idx2 != -1:
        content = content[:idx1] + channel_setup + content[idx2 + len("if let Err(e) = audio_mgr_clone.wait_for_wake_word().await {"):]

    eframe_replace = """    eframe::run_native(
        "Kiwi",
        options,
        Box::new(move |_cc| Ok(Box::new(kiwi::gui::KiwiGui::new(gui_rx, gui_event_tx_clone)))),
    )?;"""

    idx_eframe = content.find("    eframe::run_native(")
    idx_end = content.find("    )?;", idx_eframe)
    if idx_eframe != -1 and idx_end != -1:
        content = content[:idx_eframe] + eframe_replace + content[idx_end + len("    )?;"):]

    with open("src/main.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify()
