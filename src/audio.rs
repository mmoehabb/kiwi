use async_trait::async_trait;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use futures_util::StreamExt;
use kokoro_en::KokoroTts;
use rubato::audioadapter_buffers::direct::SequentialSliceOfVecs;
use rubato::{
    Async, FixedAsync, Resampler, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};

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
    kokoro: Arc<KokoroTts>,
    tts_voice_name: String,
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
        // 3. Initialize Kokoro TTS
        let kokoro_model_path = base_path.join(
            config
                .app
                .tts_model_url
                .split('/')
                .next_back()
                .unwrap_or("kokoro-model.onnx"),
        );
        let voices_dir = base_path.join("voices");
        let default_voice_path = voices_dir.join(format!("{}.bin", config.app.tts_voice_name));

        if !kokoro_model_path.exists() {
            println!(
                "Downloading Kokoro model to {}...",
                kokoro_model_path.display()
            );
            Self::download_file(&config.app.tts_model_url, &kokoro_model_path).await?;
        }

        if !voices_dir.exists() {
            std::fs::create_dir_all(&voices_dir)
                .map_err(|e| format!("Failed to create voices directory: {}", e))?;
        }

        if !default_voice_path.exists() {
            println!(
                "Downloading default Kokoro voice to {}...",
                default_voice_path.display()
            );
            Self::download_file(&config.app.tts_voice_url, &default_voice_path).await?;
        }

        let kokoro = KokoroTts::new(&kokoro_model_path, &voices_dir)
            .await
            .map_err(|e| format!("Failed to load Kokoro model: {:?}", e))?;

        Ok(Self {
            whisper_ctx: Arc::new(whisper_ctx),
            kokoro: Arc::new(kokoro),
            tts_voice_name: config.app.tts_voice_name.clone(),
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
                                // TODO: Let the user choose the microphone channel.
                                let mono_sample = frame[0];
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
                                // TODO: Let the user choose the microphone channel.
                                let mono_sample = frame[0] as f32 / i16::MAX as f32;
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

            let mut resampler = if input_sample_rate != target_sample_rate {
                let params = SincInterpolationParameters {
                    sinc_len: 256,
                    f_cutoff: 0.95,
                    interpolation: SincInterpolationType::Linear,
                    oversampling_factor: 256,
                    window: WindowFunction::BlackmanHarris2,
                };
                let chunk_size =
                    (input_sample_rate as f32 * (chunk_duration_ms as f32 / 1000.0)) as usize;
                Some(
                    Async::<f32>::new_sinc(
                        target_sample_rate as f64 / input_sample_rate as f64,
                        2.0,
                        &params,
                        chunk_size,
                        1,
                        FixedAsync::Input,
                    )
                    .map_err(|e| e.to_string())?,
                )
            } else {
                None
            };

            let window_size = target_sample_rate as usize * 2; // 2 seconds window
            let mut audio_buffer: Vec<f32> = Vec::with_capacity(window_size);
            let mut resampler_input_buffer = Vec::new();

            loop {
                std::thread::sleep(std::time::Duration::from_millis(chunk_duration_ms as u64));

                let mut chunk_audio = Vec::new();
                while let Some(sample) = ringbuf::traits::Consumer::try_pop(&mut cons) {
                    chunk_audio.push(sample);
                }

                if chunk_audio.is_empty() {
                    continue;
                }

                let processed_audio = if let Some(ref mut r) = resampler {
                    let mut output = Vec::new();
                    resampler_input_buffer.extend_from_slice(&chunk_audio);

                    while resampler_input_buffer.len() >= r.input_frames_next() {
                        let frames_to_take = r.input_frames_next();
                        let current = &resampler_input_buffer[..frames_to_take];
                        let current_vec = current.to_vec();

                        let frames_in = current_vec.len();
                        let wrapped_vecs = [current_vec];
                        let adapter = SequentialSliceOfVecs::new(&wrapped_vecs, 1, frames_in)
                            .map_err(|e| e.to_string())?;

                        use rubato::audioadapter_buffers::owned::InterleavedOwned;

                        let frames = r.output_frames_next();
                        let mut buffer_out = InterleavedOwned::<f32>::new(0.0, 1, frames);
                        let (_, out_len) = r
                            .process_into_buffer(&adapter, &mut buffer_out, None)
                            .map_err(|e: rubato::ResampleError| e.to_string())?;
                        let out = buffer_out;
                        use rubato::audioadapter::Adapter;
                        let mut temp = vec![0.0; out_len];
                        out.copy_from_channel_to_slice(0, 0, &mut temp);
                        output.extend_from_slice(&temp);

                        // remove processed frames
                        resampler_input_buffer.drain(0..frames_to_take);
                    }
                    output
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
                                // TODO: Let the user choose the microphone channel.
                                let mono_sample = frame[0];
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
                                // TODO: Let the user choose the microphone channel.
                                let mono_sample = frame[0] as f32 / i16::MAX as f32;
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
            let params = SincInterpolationParameters {
                sinc_len: 256,
                f_cutoff: 0.95,
                interpolation: SincInterpolationType::Linear,
                oversampling_factor: 256,
                window: WindowFunction::BlackmanHarris2,
            };
            let chunk_size = 1024; // Use a fixed chunk size for processing
            let mut resampler = Async::<f32>::new_sinc(
                target_sample_rate as f64 / input_sample_rate as f64,
                2.0,
                &params,
                chunk_size,
                1,
                FixedAsync::Input,
            )
            .map_err(|e| e.to_string())?;

            let mut output = Vec::new();
            let mut input = audio_data.as_slice();
            while !input.is_empty() {
                let frames_to_take = std::cmp::min(input.len(), resampler.input_frames_next());
                let (current, next) = input.split_at(frames_to_take);
                let current_vec = current.to_vec();
                let frames_in = current_vec.len();
                let wrapped_vecs = [current_vec];
                let adapter = SequentialSliceOfVecs::new(&wrapped_vecs, 1, frames_in)
                    .map_err(|e| e.to_string())?;

                let partial = if next.is_empty() {
                    Some(frames_in)
                } else {
                    None
                };
                let indexing = rubato::Indexing {
                    input_offset: 0,
                    output_offset: 0,
                    partial_len: partial,
                    active_channels_mask: None,
                };

                use rubato::audioadapter_buffers::owned::InterleavedOwned;

                let frames = resampler.output_frames_next();
                let mut buffer_out = InterleavedOwned::<f32>::new(0.0, 1, frames);
                let (_, out_len) = resampler
                    .process_into_buffer(&adapter, &mut buffer_out, Some(&indexing))
                    .map_err(|e: rubato::ResampleError| e.to_string())?;
                let out = buffer_out;
                use rubato::audioadapter::Adapter;
                let mut temp = vec![0.0; out_len];
                out.copy_from_channel_to_slice(0, 0, &mut temp);
                output.extend_from_slice(&temp);
                input = next;
            }
            output
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
        let text_owned = text.to_string();

        let (audio_data, _duration) = self
            .kokoro
            .synth(text_owned, &self.tts_voice_name)
            .await
            .map_err(|e| format!("Kokoro TTS error: {:?}", e))?;

        Ok(audio_data)
    }
}
