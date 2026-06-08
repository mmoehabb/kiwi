cat << 'PY' > patch_config.py
import sys

def modify_config():
    with open("src/config.rs", "r") as f:
        content = f.read()

    new_method = """
    pub fn wakeword_templates_path() -> Result<PathBuf, String> {
        let mut path = dirs::data_local_dir().ok_or("Could not find user local data directory")?;
        path.push("kiwi");
        path.push("models");
        if !path.exists() {
            std::fs::create_dir_all(&path).map_err(|e| format!("Failed to create models dir: {}", e))?;
        }
        path.push("wakeword_templates.bin");
        Ok(path)
    }

    pub fn load(&mut self) -> Result<(), String> {"""

    content = content.replace("    pub fn load(&mut self) -> Result<(), String> {", new_method)

    with open("src/config.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify_config()
PY
python3 patch_config.py

cat << 'PY' > patch_gui.py
import sys

def modify_gui():
    with open("src/gui.rs", "r") as f:
        content = f.read()

    content = content.replace(
        "    Speaking,\n}",
        "    Speaking,\n    /// Mascot is in onboarding state, prompting user to record wakeword.\n    Onboarding,\n}"
    )

    new_struct_fields = """    position_set: bool,

    // Onboarding UI state
    pub templates_recorded: usize,
    pub is_recording: bool,
    pub tx_gui: Option<mpsc::Sender<GuiEvent>>,
}

#[derive(Debug, Clone)]
pub enum GuiEvent {
    RecordSample,
    DoneOnboarding,
}
"""
    content = content.replace("    position_set: bool,\n}", new_struct_fields)

    new_new_method = """    pub fn new(rx: mpsc::Receiver<MascotState>, tx_gui: mpsc::Sender<GuiEvent>) -> Self {
        Self {
            state: MascotState::Idle,
            rx,
            idle_texture: None,
            listening_texture: None,
            thinking_texture: None,
            speaking_texture: None,
            position_set: false,
            templates_recorded: 0,
            is_recording: false,
            tx_gui: Some(tx_gui),
        }
    }"""
    content = content.replace("    pub fn new(rx: mpsc::Receiver<MascotState>) -> Self {\n        Self {\n            state: MascotState::Idle,\n            rx,\n            idle_texture: None,\n            listening_texture: None,\n            thinking_texture: None,\n            speaking_texture: None,\n            position_set: false,\n        }\n    }", new_new_method)

    new_match = """        let texture = match self.state {
            MascotState::Idle | MascotState::Onboarding => &self.idle_texture,
            MascotState::Listening => &self.listening_texture,
            MascotState::Thinking => &self.thinking_texture,
            MascotState::Speaking => &self.speaking_texture,
        };"""
    content = content.replace("""        let texture = match self.state {
            MascotState::Idle => &self.idle_texture,
            MascotState::Listening => &self.listening_texture,
            MascotState::Thinking => &self.thinking_texture,
            MascotState::Speaking => &self.speaking_texture,
        };""", new_match)

    new_ui_logic = """        if let Some(tex) = texture {
            ui.add(egui::Image::new(tex));
        } else {
            // Fallback text if images fail to load
            ui.heading("Kiwi 🦜");
            ui.label(format!("State: {:?}", self.state));
        }

        if self.state == MascotState::Onboarding {
            egui::Window::new("Welcome to Kiwi!")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label("Kiwi needs to learn your wake word.");
                    ui.label(format!("Samples recorded: {}/3", self.templates_recorded));

                    if self.is_recording {
                        ui.label("Recording... Please speak your wake word.");
                    } else if self.templates_recorded < 3 {
                        if ui.button("Record Sample").clicked() {
                            self.is_recording = true;
                            if let Some(tx) = &self.tx_gui {
                                let _ = tx.blocking_send(GuiEvent::RecordSample);
                            }
                        }
                    } else {
                        if ui.button("Done").clicked() {
                            if let Some(tx) = &self.tx_gui {
                                let _ = tx.blocking_send(GuiEvent::DoneOnboarding);
                            }
                        }
                    }
                });
        }"""
    content = content.replace("""        if let Some(tex) = texture {
            ui.add(egui::Image::new(tex));
        } else {
            // Fallback text if images fail to load
            ui.heading("Kiwi 🦜");
            ui.label(format!("State: {:?}", self.state));
        }""", new_ui_logic)

    with open("src/gui.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify_gui()
PY
python3 patch_gui.py

cat << 'PY' > patch_audio_5.py
import sys

def modify():
    with open("src/audio.rs", "r") as f:
        content = f.read()

    new_imports = """
use std::time::Duration;
use std::sync::Arc;
use crate::wakeword::WakewordEngine;

#[async_trait::async_trait]
pub trait WakeWordListener {
    async fn wait_for_wake_word(&self, engine: Arc<tokio::sync::Mutex<WakewordEngine>>) -> Result<(), String>;
}
"""
    content = content.replace("use std::time::Duration;\n\n#[async_trait::async_trait]\npub trait WakeWordListener {\n    async fn wait_for_wake_word(&self) -> Result<(), String>;\n}", new_imports)

    start_str = "impl WakeWordListener for AudioManager {"
    end_str = "    }\n}\n\n#[async_trait::async_trait]\nimpl SpeechToText for AudioManager {"

    start_idx = content.find(start_str)
    end_idx = content.find(end_str, start_idx)

    new_func = """impl WakeWordListener for AudioManager {
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
                    // We have at least 1 second of audio
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

    content = content[:start_idx] + new_func + content[end_idx:]

    with open("src/audio.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify()
PY
python3 patch_audio_5.py

cat << 'PY' > patch_main_rebuild.py
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
PY
python3 patch_main_rebuild.py

cargo check
