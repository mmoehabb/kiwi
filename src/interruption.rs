use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::HeapRb;
use ringbuf::traits::{Consumer, Producer, Split};
use std::time::Duration;

pub struct InterruptionDetector {
    silence_threshold: f32,
}

impl InterruptionDetector {
    pub fn new(silence_threshold: f32) -> Self {
        Self { silence_threshold }
    }

    pub async fn wait_for_interruption(
        &self,
        cancel_rx: tokio::sync::watch::Receiver<bool>,
    ) -> Result<(Vec<f32>, u32), String> {
        let silence_threshold = self.silence_threshold;

        let res = tokio::task::spawn_blocking(move || {
            let host = cpal::default_host();
            let device = host
                .default_input_device()
                .ok_or("Failed to get default input device")?;

            let config = device.default_input_config().map_err(|e| e.to_string())?;
            let channels = config.channels();
            let input_sample_rate = config.sample_rate().0;

            // Short buffer just for checking RMS
            let rb = HeapRb::<f32>::new(input_sample_rate as usize * 2);
            let (mut prod, mut cons) = rb.split();

            let mut recorded_audio = Vec::new();

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
            let required_silence_duration_secs = 2.0;
            let required_silent_chunks =
                (required_silence_duration_secs * 1000.0 / chunk_duration_ms as f32) as usize;

            let mut silent_chunks = 0;

            // First wait for sound
            loop {
                if *cancel_rx.borrow() {
                    return Err("Cancelled".to_string());
                }

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

                    if rms > silence_threshold {
                        recorded_audio.extend(chunk_audio);
                        break;
                    }
                }
            }

            // Then record until silence
            loop {
                if *cancel_rx.borrow() {
                    return Err("Cancelled".to_string());
                }

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

                    recorded_audio.extend(chunk_audio);

                    if rms < silence_threshold {
                        silent_chunks += 1;
                    } else {
                        silent_chunks = 0;
                    }

                    if silent_chunks >= required_silent_chunks {
                        break;
                    }
                }
            }

            stream.pause().map_err(|e| e.to_string())?;

            while let Some(sample) = cons.try_pop() {
                recorded_audio.push(sample);
            }

            Ok::<(Vec<f32>, u32), String>((recorded_audio, input_sample_rate))
        })
        .await
        .map_err(|e| e.to_string())??;

        Ok(res)
    }
}
