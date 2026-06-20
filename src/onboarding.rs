use crate::gui::{GuiEvent, MascotState};
use crate::wakeword::WakewordEngine;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

pub async fn run_onboarding(
    gui_tx_clone: mpsc::Sender<MascotState>,
    mut gui_event_rx: mpsc::Receiver<GuiEvent>,
    wakeword_engine_arc_clone: Arc<Mutex<WakewordEngine>>,
) {
    let _ = gui_tx_clone
        .send(MascotState::Onboarding {
            recorded: 0,
            is_recording: false,
        })
        .await;
    let mut recorded = 0;
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
                                        // TODO: Let the user choose the microphone channel.
                                        let mono_sample = frame[0];
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
                        cpal::SampleFormat::I16 => device
                            .build_input_stream(
                                &conf.clone().into(),
                                move |data: &[i16], _| {
                                    for frame in data.chunks(channels as usize) {
                                        // TODO: Let the user choose the microphone channel.
                                        let mono_sample = frame[0] as f32 / i16::MAX as f32;
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
                    use rubato::audioadapter_buffers::direct::SequentialSliceOfVecs;
                    use rubato::{
                        Async, FixedAsync, Resampler, SincInterpolationParameters,
                        SincInterpolationType, WindowFunction,
                    };

                    let params = SincInterpolationParameters {
                        sinc_len: 256,
                        f_cutoff: 0.95,
                        interpolation: SincInterpolationType::Linear,
                        oversampling_factor: 256,
                        window: WindowFunction::BlackmanHarris2,
                    };

                    let chunk_size = 1024;
                    let mut resampler = Async::<f32>::new_sinc(
                        16000.0 / _rate as f64,
                        2.0,
                        &params,
                        chunk_size,
                        1,
                        FixedAsync::Input,
                    )
                    .unwrap();

                    let mut output = Vec::new();
                    let mut input = audio_data.as_slice();
                    while !input.is_empty() {
                        let frames_to_take =
                            std::cmp::min(input.len(), resampler.input_frames_next());
                        let (current, next) = input.split_at(frames_to_take);
                        let current_vec = current.to_vec();
                        let frames_in = current_vec.len();
                        let wrapped_vecs = [current_vec];
                        let adapter =
                            SequentialSliceOfVecs::new(&wrapped_vecs, 1, frames_in).unwrap();

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
                            .unwrap();
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

                cached_raw_audio.push(processed.clone());
                let mut engine = wakeword_engine_arc_clone.lock().await;
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
                    let mut engine = wakeword_engine_arc_clone.lock().await;
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
                let engine = wakeword_engine_arc_clone.lock().await;
                let _ = engine.save_templates();
                let _ = gui_tx_clone.send(MascotState::Idle).await;
                break;
            }
        }
    }
}
