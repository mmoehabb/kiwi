use async_trait::async_trait;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use dasp::{Signal, interpolate::linear::Linear, signal};
use futures_util::StreamExt;
use pocket_tts::TTSModel;

#[async_trait::async_trait]
pub trait WakeWordListener {
    async fn wait_for_wake_word(
        &self,
        engine: std::sync::Arc<tokio::sync::Mutex<crate::wakeword::WakewordEngine>>,
    ) -> Result<(), String>;
}

#[async_trait]
pub trait SpeechToText {
    async fn listen_and_transcribe(&self) -> Result<String, String>;
    async fn transcribe_audio(
        &self,
        audio_data: Vec<f32>,
        input_sample_rate: u32,
    ) -> Result<String, String>;
}

#[async_trait]
pub trait TextToSpeech {
    async fn speak(&self, text: &str) -> Result<Vec<f32>, String>;
}
use crate::config::Configuration;
use reqwest;
use ringbuf::HeapRb;
use ringbuf::traits::{Consumer, Producer, Split};
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;
use whisper_rs::{WhisperContext, WhisperContextParameters};

/// The unified manager for all audio operations.
pub struct AudioManager {
    whisper_ctx: Arc<WhisperContext>,
    pocket_model: Arc<TTSModel>,
    voice_state: Arc<pocket_tts::ModelState>,
}

impl AudioManager {
    pub async fn new(config: Arc<Configuration>) -> Result<Self, String> {
        let base_path = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("kiwi/models");

        if !base_path.exists() {
            std::fs::create_dir_all(&base_path)
                .map_err(|e| format!("Failed to create model directory: {}", e))?;
        }

        // 1. Initialize Whisper STT (Base model)
        let whisper_model_path = base_path.join(
            config
                .app
                .stt_model_url
                .split('/')
                .next_back()
                .unwrap_or("ggml-base.en.bin"),
        );
        if !whisper_model_path.exists() {
            println!(
                "Downloading Whisper model to {}...",
                whisper_model_path.display()
            );
            Self::download_file(&config.app.stt_model_url, &whisper_model_path).await?;
            println!("Whisper model downloaded successfully.");
        }
        let whisper_ctx = WhisperContext::new_with_params(
            whisper_model_path.to_str().unwrap(),
            WhisperContextParameters::default(),
        )
        .map_err(|e| format!("Failed to load Whisper model: {}", e))?;

        // 2. Initialize Whisper for Wake Word (Tiny model for faster inference)
        // TODO: Replace this with a native Rust wake word engine in the future.
        // 3. Initialize Pocket TTS (non-gated repo)
        let weights_path = base_path.join("pocket-tts-weights.safetensors");
        let tokenizer_path = base_path.join("pocket-tts-tokenizer.model");

        if !weights_path.exists() {
            println!("Downloading Pocket TTS weights...");
            Self::download_file(
                "https://huggingface.co/kyutai/pocket-tts-without-voice-cloning/resolve/main/tts_b6369a24.safetensors",
                &weights_path,
            ).await?;
            println!("Pocket TTS weights downloaded.");
        }

        if !tokenizer_path.exists() {
            println!("Downloading Pocket TTS tokenizer...");
            Self::download_file(
                "https://huggingface.co/kyutai/pocket-tts-without-voice-cloning/resolve/main/tokenizer.model",
                &tokenizer_path,
            ).await?;
            println!("Pocket TTS tokenizer downloaded.");
        }

        let config_yaml = include_str!("pocket_tts_config.yaml");
        let weights_bytes = std::fs::read(&weights_path)
            .map_err(|e| format!("Failed to read Pocket TTS weights: {}", e))?;
        let tokenizer_bytes = std::fs::read(&tokenizer_path)
            .map_err(|e| format!("Failed to read Pocket TTS tokenizer: {}", e))?;

        let pocket_model =
            TTSModel::load_from_bytes(config_yaml.as_bytes(), &weights_bytes, &tokenizer_bytes)
                .map_err(|e| format!("Failed to load Pocket TTS model: {:?}", e))?;

        let voice_embedding_path = base_path.join("embeddings");
        let voice_path = if config.app.tts_voice_path.is_empty() {
            let cosette_path = voice_embedding_path.join("cosette.safetensors");
            if !cosette_path.exists() {
                std::fs::create_dir_all(&voice_embedding_path)
                    .map_err(|e| format!("Failed to create embeddings dir: {}", e))?;
                println!("Downloading Pocket TTS voice 'cosette'...");
                Self::download_file(
                    "https://huggingface.co/kyutai/pocket-tts-without-voice-cloning/resolve/main/embeddings/cosette.safetensors",
                    &cosette_path,
                ).await?;
                println!("Voice 'cosette' downloaded.");
            }
            cosette_path
        } else {
            std::path::PathBuf::from(&config.app.tts_voice_path)
        };

        let voice_state = if voice_path.extension().is_some_and(|e| e == "safetensors") {
            pocket_model
                .get_voice_state_from_prompt_file(&voice_path)
                .map_err(|e| format!("Failed to load voice from {:?}: {:?}", voice_path, e))?
        } else {
            pocket_model
                .get_voice_state(&voice_path)
                .map_err(|e| format!("Failed to load voice from {:?}: {:?}", voice_path, e))?
        };

        Ok(Self {
            whisper_ctx: Arc::new(whisper_ctx),
            pocket_model: Arc::new(pocket_model),
            voice_state: Arc::new(voice_state),
        })
    }

    async fn download_file(url: &str, path: &std::path::Path) -> Result<(), String> {
        let response = reqwest::get(url).await.map_err(|e| e.to_string())?;
        if !response.status().is_success() {
            return Err(format!("Failed to download {}: {}", url, response.status()));
        }

        let mut file = File::create(path).map_err(|e| e.to_string())?;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| e.to_string())?;
            file.write_all(&chunk).map_err(|e| e.to_string())?;
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl WakeWordListener for AudioManager {
    async fn wait_for_wake_word(
        &self,
        engine: std::sync::Arc<tokio::sync::Mutex<crate::wakeword::WakewordEngine>>,
    ) -> Result<(), String> {
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

            let rb = ringbuf::HeapRb::<f32>::new(input_sample_rate as usize * 5); // 5 seconds buffer
            let (mut prod, mut cons) = ringbuf::traits::Split::split(rb);

            let stream = match config.sample_format() {
                cpal::SampleFormat::F32 => device
                    .build_input_stream(
                        &config.into(),
                        move |data: &[f32], _: &cpal::InputCallbackInfo| {
                            for frame in data.chunks(channels as usize) {
                                let mono_sample = frame.iter().sum::<f32>() / channels as f32;
                                let _ = ringbuf::traits::Producer::try_push(&mut prod, mono_sample);
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
                                let _ = ringbuf::traits::Producer::try_push(&mut prod, mono_sample);
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
                std::thread::sleep(std::time::Duration::from_millis(chunk_duration_ms as u64));

                let mut chunk_audio = Vec::new();
                while let Some(sample) = ringbuf::traits::Consumer::try_pop(&mut cons) {
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

        Ok(())
    }
}

#[async_trait::async_trait]
impl SpeechToText for AudioManager {
    async fn listen_and_transcribe(&self) -> Result<String, String> {
        let max_recording_duration_secs = 15;
        let silence_threshold = 0.02; // TODO: estimate the silence threshold periodically in the future
        let initial_silence_duration_secs = 5.0;
        let required_silence_duration_secs = 2.0;

        let (audio_data, input_sample_rate) = tokio::task::spawn_blocking(move || {
            let host = cpal::default_host();
            let device = host
                .default_input_device()
                .ok_or("Failed to get default input device")?;

            let config = device.default_input_config().map_err(|e| e.to_string())?;
            let channels = config.channels();
            let input_sample_rate = config.sample_rate().0;

            let rb = HeapRb::<f32>::new(input_sample_rate as usize * max_recording_duration_secs);
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

            let chunk_duration_ms = 100;
            let max_iterations = (max_recording_duration_secs * 1000) / chunk_duration_ms;
            let mut silent_chunks = 0;
            let required_silent_chunks =
                (required_silence_duration_secs * 1000.0 / chunk_duration_ms as f32) as usize;
            let initial_silent_chunks =
                (initial_silence_duration_secs * 1000.0 / chunk_duration_ms as f32) as usize;

            let mut all_audio_data = Vec::new();
            let mut has_spoken = false;

            for _ in 0..max_iterations {
                std::thread::sleep(Duration::from_millis(chunk_duration_ms as u64));

                let mut chunk_audio = Vec::new();
                while let Some(sample) = cons.try_pop() {
                    chunk_audio.push(sample);
                }

                if !chunk_audio.is_empty() {
                    let mut sum_squares = 0.0;
                    for &sample in &chunk_audio {
                        sum_squares += sample * sample;
                    }
                    let rms = (sum_squares / chunk_audio.len() as f32).sqrt();

                    if rms < silence_threshold {
                        silent_chunks += 1;
                    } else {
                        silent_chunks = 0;
                        has_spoken = true;
                    }

                    all_audio_data.extend(chunk_audio);

                    if !has_spoken && silent_chunks >= initial_silent_chunks {
                        all_audio_data.clear();
                        break;
                    }

                    if has_spoken && silent_chunks >= required_silent_chunks {
                        break;
                    }
                }
            }

            stream.pause().map_err(|e| e.to_string())?;

            while let Some(sample) = cons.try_pop() {
                all_audio_data.push(sample);
            }

            Ok::<(Vec<f32>, u32), String>((all_audio_data, input_sample_rate))
        })
        .await
        .map_err(|e| e.to_string())??;

        if audio_data.is_empty() {
            return Ok("".to_string());
        }

        self.transcribe_audio(audio_data, input_sample_rate).await
    }

    async fn transcribe_audio(
        &self,
        audio_data: Vec<f32>,
        input_sample_rate: u32,
    ) -> Result<String, String> {
        let target_sample_rate = 16000;
        let processed_audio = if input_sample_rate != target_sample_rate {
            let mut signal = signal::from_iter(audio_data.clone());
            let interp = Linear::new(signal.next(), signal.next());
            // Need to know how many samples to take.
            let samples_to_take = (audio_data.len() as f64
                * (target_sample_rate as f64 / input_sample_rate as f64))
                as usize;
            signal
                .from_hz_to_hz(interp, input_sample_rate as f64, target_sample_rate as f64)
                .take(samples_to_take)
                .collect::<Vec<f32>>()
        } else {
            audio_data
        };

        // 2. Process with Whisper
        let ctx = self.whisper_ctx.clone();

        let transcribed_text = tokio::task::spawn_blocking(move || {
            let mut state = ctx.create_state().map_err(|e| e.to_string())?;

            let mut params =
                whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
            params.set_language(Some("en"));
            params.set_print_special(false);
            params.set_print_progress(false);
            params.set_print_realtime(false);
            params.set_print_timestamps(false);

            state
                .full(params, &processed_audio)
                .map_err(|e| e.to_string())?;

            let num_segments = state.full_n_segments();
            let mut full_text = String::new();

            for i in 0..num_segments {
                if let Some(segment) = state.get_segment(i)
                    && let Ok(text) = segment.to_str()
                {
                    full_text.push_str(text);
                }
            }

            Ok::<String, String>(full_text.trim().to_string())
        })
        .await
        .map_err(|e| e.to_string())??;

        Ok(transcribed_text)
    }
}

#[async_trait::async_trait]
impl TextToSpeech for AudioManager {
    async fn speak(&self, text: &str) -> Result<Vec<f32>, String> {
        let audio_tensor = self
            .pocket_model
            .generate(text, &self.voice_state)
            .map_err(|e| format!("Pocket TTS error: {:?}", e))?;

        let audio_data: Vec<f32> = audio_tensor
            .squeeze(0)
            .map_err(|e| format!("Failed to convert TTS output: {:?}", e))?
            .to_vec1::<f32>()
            .map_err(|e| format!("Failed to convert TTS output: {:?}", e))?;

        Ok(audio_data)
    }
}
