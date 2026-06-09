import sys
import re

def modify():
    with open("src/main.rs", "r") as f:
        content = f.read()

    old_loop = """            let mut recorded = 0;
            while let Some(event) = gui_event_rx.recv().await {
                match event {
                    GuiEvent::RecordSample => {
                        let _ = gui_tx_clone
                            .send(MascotState::Onboarding {
                                recorded,
                                is_recording: true,
                            })
                            .await;
                        let (audio_data, _rate) = tokio::task::spawn_blocking(|| {
                            let host = cpal::default_host();
                            use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
                            let device = host.default_input_device().unwrap();
                            let conf = device.default_input_config().unwrap();
                            let channels = conf.channels();
                            let rb = ringbuf::HeapRb::<f32>::new(conf.sample_rate().0 as usize * 5);
                            let (mut prod, mut cons) = ringbuf::traits::Split::split(rb);
                            let stream = match conf.sample_format() {
                                cpal::SampleFormat::F32 => device
                                    .build_input_stream(
                                        &conf.clone().into(),
                                        move |data: &[f32], _| {
                                            for frame in data.chunks(channels as usize) {
                                                let mono_sample =
                                                    frame.iter().sum::<f32>() / channels as f32;
                                                let _ = ringbuf::traits::Producer::try_push(
                                                    &mut prod,
                                                    mono_sample,
                                                );
                                            }
                                        },
                                        |err| eprintln!("error: {}", err),
                                        None,
                                    )
                                    .unwrap(),
                                _ => panic!("Unsupported format"),
                            };
                            stream.play().unwrap();
                            std::thread::sleep(std::time::Duration::from_millis(2000));
                            stream.pause().unwrap();
                            let mut buf = Vec::new();
                            while let Some(s) = ringbuf::traits::Consumer::try_pop(&mut cons) {
                                buf.push(s);
                            }
                            (buf, conf.sample_rate().0)
                        })
                        .await
                        .unwrap();
                        let processed = if _rate != 16000 {
                            use dasp::{interpolate::linear::Linear, signal, Signal};
                            let mut sig = signal::from_iter(audio_data.clone());
                            let interp = Linear::new(sig.next(), sig.next());
                            sig.from_hz_to_hz(interp, _rate as f64, 16000.0)
                                .take((audio_data.len() as f64 * (16000.0 / _rate as f64)) as usize)
                                .collect()
                        } else {
                            audio_data
                        };
                        let mut engine: tokio::sync::MutexGuard<kiwi::wakeword::WakewordEngine> =
                            wakeword_engine_arc_clone.lock().await;
                        engine.add_template(&processed);
                        recorded += 1;
                        let _ = gui_tx_clone
                            .send(MascotState::Onboarding {
                                recorded,
                                is_recording: false,
                            })
                            .await;
                    }
                    GuiEvent::DoneOnboarding => {
                        let engine = wakeword_engine_arc_clone.lock().await;
                        let _ = engine.save_templates();
                        let _ = gui_tx_clone.send(MascotState::Idle).await;
                        break;
                    }
                }
            }"""

    new_loop = """            let mut recorded = 0;
            let mut cached_raw_audio: Vec<Vec<f32>> = Vec::new();
            while let Some(event) = gui_event_rx.recv().await {
                match event {
                    GuiEvent::RecordSample => {
                        let _ = gui_tx_clone
                            .send(MascotState::Onboarding {
                                recorded,
                                is_recording: true,
                            })
                            .await;
                        let (audio_data, _rate) = tokio::task::spawn_blocking(|| {
                            let host = cpal::default_host();
                            use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
                            let device = host.default_input_device().unwrap();
                            let conf = device.default_input_config().unwrap();
                            let channels = conf.channels();
                            let rb = ringbuf::HeapRb::<f32>::new(conf.sample_rate().0 as usize * 5);
                            let (mut prod, mut cons) = ringbuf::traits::Split::split(rb);
                            let stream = match conf.sample_format() {
                                cpal::SampleFormat::F32 => device
                                    .build_input_stream(
                                        &conf.clone().into(),
                                        move |data: &[f32], _| {
                                            for frame in data.chunks(channels as usize) {
                                                let mono_sample =
                                                    frame.iter().sum::<f32>() / channels as f32;
                                                let _ = ringbuf::traits::Producer::try_push(
                                                    &mut prod,
                                                    mono_sample,
                                                );
                                            }
                                        },
                                        |err| eprintln!("error: {}", err),
                                        None,
                                    )
                                    .unwrap(),
                                _ => panic!("Unsupported format"),
                            };
                            stream.play().unwrap();
                            std::thread::sleep(std::time::Duration::from_millis(2000));
                            stream.pause().unwrap();
                            let mut buf = Vec::new();
                            while let Some(s) = ringbuf::traits::Consumer::try_pop(&mut cons) {
                                buf.push(s);
                            }
                            (buf, conf.sample_rate().0)
                        })
                        .await
                        .unwrap();
                        let processed = if _rate != 16000 {
                            use dasp::{interpolate::linear::Linear, signal, Signal};
                            let mut sig = signal::from_iter(audio_data.clone());
                            let interp = Linear::new(sig.next(), sig.next());
                            sig.from_hz_to_hz(interp, _rate as f64, 16000.0)
                                .take((audio_data.len() as f64 * (16000.0 / _rate as f64)) as usize)
                                .collect()
                        } else {
                            audio_data
                        };

                        cached_raw_audio.push(processed.clone());
                        let mut engine: tokio::sync::MutexGuard<kiwi::wakeword::WakewordEngine> =
                            wakeword_engine_arc_clone.lock().await;
                        engine.add_template(&processed);
                        recorded += 1;
                        let _ = gui_tx_clone
                            .send(MascotState::Onboarding {
                                recorded,
                                is_recording: false,
                            })
                            .await;
                    }
                    GuiEvent::PlaySample(idx) => {
                        if idx < cached_raw_audio.len() {
                            let audio = cached_raw_audio[idx].clone();
                            tokio::task::spawn_blocking(move || {
                                let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
                                let sink = rodio::Sink::try_new(&stream_handle).unwrap();
                                let buffer = rodio::buffer::SamplesBuffer::new(1, 16000, audio);
                                sink.append(buffer);
                                sink.sleep_until_end();
                            });
                        }
                    }
                    GuiEvent::DeleteSample(idx) => {
                        if idx < cached_raw_audio.len() {
                            cached_raw_audio.remove(idx);
                            let mut engine: tokio::sync::MutexGuard<kiwi::wakeword::WakewordEngine> =
                                wakeword_engine_arc_clone.lock().await;
                            engine.remove_template(idx);
                            recorded -= 1;
                            let _ = gui_tx_clone
                                .send(MascotState::Onboarding {
                                    recorded,
                                    is_recording: false,
                                })
                                .await;
                        }
                    }
                    GuiEvent::DoneOnboarding => {
                        if recorded >= 3 {
                            let engine = wakeword_engine_arc_clone.lock().await;
                            let _ = engine.save_templates();
                            let _ = gui_tx_clone.send(MascotState::Idle).await;
                            break;
                        }
                    }
                }
            }"""

    content = content.replace(old_loop, new_loop)

    with open("src/main.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify()
