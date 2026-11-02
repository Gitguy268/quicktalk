use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
    time::{Duration, Instant},
};
use tauri::Emitter;

pub struct Recording {
    pub stop: Arc<AtomicBool>,
    pub handle: thread::JoinHandle<Result<f64, String>>,
    pub id: String,
}
pub fn devices() -> Result<Vec<String>, String> {
    cpal::default_host()
        .input_devices()
        .map_err(|e| e.to_string())?
        .map(|d| d.name().map_err(|e| e.to_string()))
        .collect()
}
pub fn start(
    path: PathBuf,
    device_name: String,
    app: tauri::AppHandle,
    id: String,
) -> Result<Recording, String> {
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let (ready_tx, ready_rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let run = || -> Result<f64, String> {
            let host = cpal::default_host();
            let system = device_name == "__system__";
            let device = if system {
                host.default_output_device()
            } else if device_name.is_empty() {
                host.default_input_device()
            } else {
                host.input_devices()
                    .ok()
                    .and_then(|mut ds| ds.find(|d| d.name().unwrap_or_default() == device_name))
            }
            .ok_or("No microphone available. Select an input device in Settings.")?;
            let config = if system {
                device.default_output_config()
            } else {
                device.default_input_config()
            }
            .map_err(|e| e.to_string())?;
            let rate = config.sample_rate().0;
            let channels = config.channels() as usize;
            let (tx, rx) = mpsc::sync_channel::<Vec<i16>>(128);
            let failed = Arc::new(AtomicBool::new(false));
            let error_flag = failed.clone();
            let error = move |_err: cpal::StreamError| {
                error_flag.store(true, Ordering::Relaxed);
            };
            let overflow = failed.clone();
            let stream = match config.sample_format() {
                cpal::SampleFormat::F32 => device.build_input_stream(
                    &config.clone().into(),
                    move |data: &[f32], _| {
                        let v = data
                            .chunks(channels)
                            .map(|s| {
                                ((s.iter().sum::<f32>() / channels as f32).clamp(-1., 1.) * 32767.)
                                    as i16
                            })
                            .collect();
                        if tx.try_send(v).is_err() {
                            overflow.store(true, Ordering::Relaxed);
                        }
                    },
                    error,
                    None,
                ),
                cpal::SampleFormat::I16 => device.build_input_stream(
                    &config.clone().into(),
                    move |data: &[i16], _| {
                        let v = data
                            .chunks(channels)
                            .map(|s| {
                                (s.iter().map(|x| *x as i32).sum::<i32>() / channels as i32) as i16
                            })
                            .collect();
                        if tx.try_send(v).is_err() {
                            overflow.store(true, Ordering::Relaxed);
                        }
                    },
                    error,
                    None,
                ),
                cpal::SampleFormat::U16 => device.build_input_stream(
                    &config.clone().into(),
                    move |data: &[u16], _| {
                        let v = data
                            .chunks(channels)
                            .map(|s| {
                                (s.iter().map(|x| *x as i32 - 32768).sum::<i32>() / channels as i32)
                                    as i16
                            })
                            .collect();
                        if tx.try_send(v).is_err() {
                            overflow.store(true, Ordering::Relaxed);
                        }
                    },
                    error,
                    None,
                ),
                _ => return Err("This microphone sample format is not supported.".into()),
            }
            .map_err(|e| format!("Microphone access failed: {e}"))?;
            let mut writer = hound::WavWriter::create(
                &path,
                hound::WavSpec {
                    channels: 1,
                    sample_rate: rate,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                },
            )
            .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            let _ = ready_tx.send(Ok(()));
            let mut count = 0u64;
            let mut meter = Instant::now();
            let mut flush = Instant::now();
            loop {
                if failed.load(Ordering::Relaxed) {
                    return Err("Audio device disconnected or recording could not keep up. The saved portion is available to retry.".into());
                }
                match rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(data) => {
                        if meter.elapsed() > Duration::from_millis(80) {
                            let level = (data
                                .iter()
                                .map(|x| (*x as f64 / 32768.).powi(2))
                                .sum::<f64>()
                                / data.len().max(1) as f64)
                                .sqrt();
                            let _ = app.emit("level", level);
                            meter = Instant::now();
                        }
                        for sample in data {
                            writer.write_sample(sample).map_err(|e| e.to_string())?;
                            count += 1;
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(_) => {}
                }
                if flush.elapsed() > Duration::from_secs(2) {
                    writer.flush().map_err(|e| e.to_string())?;
                    flush = Instant::now();
                }
                if flag.load(Ordering::Relaxed) {
                    break;
                }
            }
            drop(stream);
            for data in rx.try_iter() {
                for s in data {
                    writer.write_sample(s).map_err(|e| e.to_string())?;
                    count += 1;
                }
            }
            writer.finalize().map_err(|e| e.to_string())?;
            Ok(count as f64 / rate as f64)
        };
        let result = run();
        if let Err(e) = &result {
            let _ = ready_tx.send(Err(e.clone()));
        }
        result
    });
    match ready_rx.recv_timeout(Duration::from_secs(15)) {
        Ok(Ok(())) => Ok(Recording { stop, handle, id }),
        Ok(Err(e)) => {
            let _ = handle.join();
            Err(e)
        }
        Err(_) => {
            stop.store(true, Ordering::Relaxed);
            Err("Microphone did not start. Check microphone permission in system settings.".into())
        }
    }
}
