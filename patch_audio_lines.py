import sys

def modify():
    with open("src/audio.rs", "r") as f:
        lines = f.readlines()

    new_impl = """impl WakeWordListener for AudioManager {
    async fn wait_for_wake_word(&self, engine: std::sync::Arc<tokio::sync::Mutex<crate::wakeword::WakewordEngine>>) -> Result<(), String> {
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
"""
    # Replace lines 154 to 297 with the new code
    del lines[153:297]
    lines.insert(153, new_impl)

    with open("src/audio.rs", "w") as f:
        f.writelines(lines)

if __name__ == "__main__":
    modify()
