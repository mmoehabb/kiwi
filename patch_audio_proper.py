import sys
import re

def modify():
    with open("src/audio.rs", "r") as f:
        content = f.read()

    new_imports = "use std::time::Duration;\nuse std::sync::Arc;\nuse crate::wakeword::WakewordEngine;\n"
    content = content.replace("use std::time::Duration;\n", new_imports)

    old_trait = """#[async_trait::async_trait]
pub trait WakeWordListener {
    async fn wait_for_wake_word(&self) -> Result<(), String>;
}"""
    new_trait = """#[async_trait::async_trait]
pub trait WakeWordListener {
    async fn wait_for_wake_word(&self, engine: Arc<tokio::sync::Mutex<WakewordEngine>>) -> Result<(), String>;
}"""
    content = content.replace(old_trait, new_trait)

    old_impl = """#[async_trait::async_trait]
impl WakeWordListener for AudioManager {
    async fn wait_for_wake_word(&self) -> Result<(), String> {"""

    start_idx = content.find(old_impl)
    end_idx = content.find("    }\n}\n\n#[async_trait::async_trait]\nimpl SpeechToText for AudioManager {", start_idx)

    new_impl = """#[async_trait::async_trait]
impl WakeWordListener for AudioManager {
    async fn wait_for_wake_word(&self, engine: Arc<tokio::sync::Mutex<WakewordEngine>>) -> Result<(), String> {
        let chunk_duration_ms = 100;
        let target_sample_rate = 16000;

        let engine_clone = engine.clone();

        tokio::task::spawn_blocking(move || {
            let host = cpal::default_host();
            let device = host
                .default_input_device()
                .ok_or("Failed to get default input device")?;

            let config = device.default_input_config().map_err(|e| e.to_string())?;
            let channels = config.channels();
            let input_sample_rate = config.sample_rate().0;

            let rb = HeapRb::<f32>::new(input_sample_rate as usize * 5); // 5 seconds buffer
            let (mut prod, mut cons) = rb.split();

            let stream = match config.sample_format() {
                cpal::SampleFormat::F32 => device
                    .build_input_stream(
                        &config.into(),
                        move |data: &[f32], _: &cpal::InputCallbackInfo| {
                            for frame in data.chunks(channels as usize) {
                                let mono_sample = frame.iter().sum::<f32>() / channels as f32;
                                let _ = prod.try_push(mono_sample);
                            }
                        },
                        move |err| {
                            eprintln!("an error occurred on stream: {}", err);
                        },
                        None,
                    )
                    .map_err(|e| e.to_string())?,
                cpal::SampleFormat::I16 => device
                    .build_input_stream(
                        &config.into(),
                        move |data: &[i16], _: &cpal::InputCallbackInfo| {
                            for frame in data.chunks(channels as usize) {
                                let mono_sample = frame
                                    .iter()
                                    .map(|&s| s as f32 / i16::MAX as f32)
                                    .sum::<f32>()
                                    / channels as f32;
                                let _ = prod.try_push(mono_sample);
                            }
                        },
                        move |err| {
                            eprintln!("an error occurred on stream: {}", err);
                        },
                        None,
                    )
                    .map_err(|e| e.to_string())?,
                _ => return Err("Unsupported sample format".to_string()),
            };

            stream.play().map_err(|e| e.to_string())?;

            let window_size = target_sample_rate as usize * 2; // 2 seconds window
            let mut audio_buffer: Vec<f32> = Vec::with_capacity(window_size);

            loop {
                std::thread::sleep(Duration::from_millis(chunk_duration_ms as u64));

                let mut chunk_audio = Vec::new();
                while let Some(sample) = cons.try_pop() {
                    chunk_audio.push(sample);
                }

                if chunk_audio.is_empty() {
                    continue;
                }

                let processed_audio = if input_sample_rate != target_sample_rate {
                    let mut signal = signal::from_iter(chunk_audio.clone());
                    let interp = Linear::new(signal.next(), signal.next());
                    let samples_to_take = (chunk_audio.len() as f64
                        * (target_sample_rate as f64 / input_sample_rate as f64))
                        as usize;
                    signal
                        .from_hz_to_hz(interp, input_sample_rate as f64, target_sample_rate as f64)
                        .take(samples_to_take)
                        .collect::<Vec<f32>>()
                } else {
                    chunk_audio
                };

                audio_buffer.extend(processed_audio);
                if audio_buffer.len() > window_size {
                    let drain_count = audio_buffer.len() - window_size;
                    audio_buffer.drain(0..drain_count);
                }

                if audio_buffer.len() >= target_sample_rate as usize {
                    let detect = {
                        let engine_guard = engine_clone.blocking_lock();
                        engine_guard.detect(&audio_buffer)
                    };

                    if detect {
                        stream.pause().map_err(|e| e.to_string())?;
                        return Ok(());
                    }
                }
            }
        })
        .await
        .map_err(|e| e.to_string())??;

        Ok(())"""

    content = content[:start_idx] + new_impl + content[end_idx:]

    with open("src/audio.rs", "w") as f:
        f.write(content)

def modify_main():
    with open("src/main.rs", "r") as f:
        content = f.read()

    content = content.replace("use kiwi::gui::{KiwiGui, MascotState};", "use kiwi::gui::{KiwiGui, MascotState, GuiEvent};\nuse kiwi_core::wakeword::WakewordEngine;")

    old_init = "let config = Arc::new(Configuration::new());"
    new_init = """let config = Arc::new(Configuration::new());
    let wakeword_path = Configuration::wakeword_templates_path().unwrap();
    let mut wakeword_engine = WakewordEngine::new(wakeword_path, config.app.wake_word_sensitivity);
    let wakeword_engine_arc = Arc::new(tokio::sync::Mutex::new(wakeword_engine));
"""
    content = content.replace(old_init, new_init)

    old_spawn = "tokio::spawn(async move {"
    new_spawn = """let (gui_event_tx, mut gui_event_rx) = tokio::sync::mpsc::channel(10);
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
                                    |err| eprintln!("error: {}", err),
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
                    }
                    GuiEvent::DoneOnboarding => {
                        let engine = wakeword_engine_arc_clone.lock().await;
                        let _ = engine.save_templates();
                        let _ = gui_tx_clone.send(MascotState::Idle).await;
                        break;
                    }
                }
            }
        }
"""
    content = content.replace(old_spawn, new_spawn, 1)

    content = content.replace("if let Err(e) = audio_mgr_clone.wait_for_wake_word().await {", "if let Err(e) = audio_mgr_clone.wait_for_wake_word(wakeword_engine_arc_clone.clone()).await {")
    content = content.replace("Box::new(|_cc| Ok(Box::new(KiwiGui::new(gui_rx)))),", "Box::new(move |_cc| Ok(Box::new(KiwiGui::new(gui_rx, gui_event_tx_clone)))),")

    with open("src/main.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify()
    modify_main()
