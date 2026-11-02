use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tauri::{Emitter, Manager};
pub const MODELS: &[(&str, &str)] = &[
    ("tiny", "ggml-tiny-q5_1.bin"),
    ("base", "ggml-base-q5_1.bin"),
    ("small", "ggml-small-q5_1.bin"),
    ("turbo", "ggml-large-v3-turbo-q5_0.bin"),
];
pub fn model_path(root: &Path, id: &str) -> Result<PathBuf, String> {
    if let Some((_, file)) = MODELS.iter().find(|(m, _)| *m == id) {
        Ok(root.join("models").join(file))
    } else {
        crate::hub::model_path(root, id)
    }
}

pub fn binary(app: &tauri::AppHandle, name: &str) -> Result<PathBuf, String> {
    let name = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.into()
    };
    let candidates = [
        app.path()
            .resource_dir()
            .map_err(|e| e.to_string())?
            .join("binaries")
            .join(&name),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("binaries")
            .join(&name),
    ];
    candidates.into_iter().find(|p| p.is_file()).ok_or(format!(
        "{name} is missing. Run the engine build script, then rebuild QuickTalk."
    ))
}
pub fn download(
    root: &Path,
    id: &str,
    app: &tauri::AppHandle,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let path = model_path(root, id)?;
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let file = path.file_name().unwrap().to_string_lossy();
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(1800))
        .build()
        .map_err(|e| e.to_string())?;
    let pointer = client
        .get(format!(
            "https://huggingface.co/ggerganov/whisper.cpp/raw/main/{file}"
        ))
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .text()
        .map_err(|e| e.to_string())?;
    let expected = pointer
        .lines()
        .find_map(|l| l.strip_prefix("oid sha256:"))
        .ok_or("Could not obtain model checksum")?;
    let mut response = client
        .get(format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{file}"
        ))
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let total = response.content_length().unwrap_or(0);
    let partial = path.with_extension("partial");
    let mut out = std::fs::File::create(&partial).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buf = [0u8; 65536];
    let mut received = 0u64;
    let mut last = std::time::Instant::now();
    let result = (|| {
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err("Download cancelled".to_string());
            }
            let n = response.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            hash.update(&buf[..n]);
            out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
            received += n as u64;
            if last.elapsed() > Duration::from_millis(200) {
                let _ = app.emit("download", json!({"received":received,"total":total}));
                last = std::time::Instant::now();
            }
        }
        out.sync_all().map_err(|e| e.to_string())?;
        if format!("{:x}", hash.finalize()) != expected {
            return Err("Model checksum did not match. Please retry.".into());
        }
        drop(out);
        std::fs::rename(&partial, &path).map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(partial);
    }
    result
}
pub fn transcribe(
    app: &tauri::AppHandle,
    root: &Path,
    path: &Path,
    model: &str,
    language: &str,
    prompt: &str,
    cancel: &AtomicBool,
) -> Result<(String, Value), String> {
    let model = model_path(root, model)?;
    if !model.exists() {
        return Err("Download a speech model in Settings first.".into());
    }
    let engine = binary(app, "whisper-cli")?;
    let mut reader = hound::WavReader::open(path).map_err(|e| e.to_string())?;
    let spec = reader.spec();
    if spec.sample_format != hound::SampleFormat::Int || spec.bits_per_sample != 16 {
        return Err("Import requires a 16-bit PCM WAV file.".into());
    }
    let rate = spec.sample_rate as usize;
    let channels = spec.channels as usize;
    let total = reader.duration() as usize;
    let mut source = reader.samples::<i16>();
    let work = root
        .join("audio")
        .join(format!("job-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let result = (|| {
        let mut offset = 0usize;
        let mut texts = Vec::new();
        let mut segments = Vec::new();
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err("Transcription cancelled; recording kept for retry.".into());
            }
            let samples: Vec<i16> = source
                .by_ref()
                .take(rate * channels * 30)
                .collect::<Result<_, _>>()
                .map_err(|e| e.to_string())?;
            if samples.is_empty() {
                break;
            }
            let mono: Vec<f32> = samples
                .chunks(channels)
                .map(|s| s.iter().map(|x| *x as f32).sum::<f32>() / channels as f32)
                .collect();
            let rms =
                (mono.iter().map(|x| (*x as f64).powi(2)).sum::<f64>() / mono.len() as f64).sqrt();
            if rms < 30. {
                offset += mono.len();
                continue;
            }
            let wav = work.join("chunk.wav");
            let mut w = hound::WavWriter::create(
                &wav,
                hound::WavSpec {
                    channels: 1,
                    sample_rate: 16000,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                },
            )
            .map_err(|e| e.to_string())?;
            for n in 0..(mono.len() * 16000 / rate) {
                let pos = n as f64 * rate as f64 / 16000.;
                let i = pos as usize;
                let frac = (pos - i as f64) as f32;
                let val = mono[i] * (1. - frac) + mono[(i + 1).min(mono.len() - 1)] * frac;
                w.write_sample(val as i16).map_err(|e| e.to_string())?;
            }
            w.finalize().map_err(|e| e.to_string())?;
            let log = std::fs::File::create(work.join("engine.log")).map_err(|e| e.to_string())?;
            let mut child = Command::new(&engine)
                .args(["-m"])
                .arg(&model)
                .arg("-f")
                .arg(&wav)
                .args(["-l", language, "-oj", "-of"])
                .arg(work.join("result"))
                .args(["-t", "4", "-np", "-ng", "--prompt", prompt])
                .stdout(Stdio::null())
                .stderr(log)
                .spawn()
                .map_err(|e| e.to_string())?;
            loop {
                if cancel.load(Ordering::Relaxed) {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("Transcription cancelled; recording kept for retry.".into());
                }
                if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                    if !status.success() {
                        let detail =
                            std::fs::read_to_string(work.join("engine.log")).unwrap_or_default();
                        return Err(format!(
                            "Speech engine failed: {}",
                            detail
                                .chars()
                                .rev()
                                .take(1200)
                                .collect::<String>()
                                .chars()
                                .rev()
                                .collect::<String>()
                        ));
                    }
                    break;
                }
                std::thread::sleep(Duration::from_millis(80));
            }
            let output: Value = serde_json::from_str(
                &std::fs::read_to_string(work.join("result.json")).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            if let Some(items) = output["transcription"].as_array() {
                for item in items {
                    let text = item["text"].as_str().unwrap_or("").trim();
                    if text.is_empty() {
                        continue;
                    }
                    texts.push(text.to_string());
                    segments.push(json!({"start":offset as f64/rate as f64+item["offsets"]["from"].as_f64().unwrap_or(0.)/1000.,"end":offset as f64/rate as f64+item["offsets"]["to"].as_f64().unwrap_or(0.)/1000.,"text":text,"speaker":"Microphone"}));
                }
            }
            offset += mono.len();
            let _=app.emit("progress",json!({"message":"Transcribing locally","fraction":(offset as f64/total.max(1) as f64).min(1.)}));
        }
        Ok((texts.join(" "), Value::Array(segments)))
    })();
    let _ = std::fs::remove_dir_all(&work);
    result
}

pub fn transcribe_cloud(
    app: &tauri::AppHandle,
    root: &Path,
    path: &Path,
    endpoint: &str,
    model: &str,
    language: &str,
    cancel: &AtomicBool,
) -> Result<(String, Value), String> {
    let url =
        reqwest::Url::parse(endpoint).map_err(|_| "Enter a transcription API URL in Settings.")?;
    if url.scheme() != "https" {
        return Err("Cloud transcription requires an HTTPS endpoint.".into());
    }
    if model.trim().is_empty() {
        return Err("Enter the cloud transcription model name.".into());
    }
    let key = keyring::Entry::new("app.quicktalk.desktop", "speech-provider")
        .map_err(|e| e.to_string())?
        .get_password()
        .map_err(|_| "Save a speech API key in Settings first.")?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    let mut reader = hound::WavReader::open(path).map_err(|e| e.to_string())?;
    let spec = reader.spec();
    let mut source = reader.samples::<i16>();
    let chunk = root
        .join("audio")
        .join(format!("cloud-{}.wav", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut texts = Vec::new();
        let mut segments = Vec::new();
        let mut offset = 0.;
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err("Cloud transcription cancelled.".into());
            }
            let samples: Vec<i16> = source
                .by_ref()
                .take(spec.sample_rate as usize * spec.channels as usize * 30)
                .collect::<Result<_, _>>()
                .map_err(|e| e.to_string())?;
            if samples.is_empty() {
                break;
            }
            let duration = samples.len() as f64 / spec.channels as f64 / spec.sample_rate as f64;
            let mut writer = hound::WavWriter::create(&chunk, spec).map_err(|e| e.to_string())?;
            for sample in samples {
                writer.write_sample(sample).map_err(|e| e.to_string())?;
            }
            writer.finalize().map_err(|e| e.to_string())?;
            let mut form = reqwest::blocking::multipart::Form::new()
                .text("model", model.to_string())
                .text("response_format", "json")
                .file("file", &chunk)
                .map_err(|e| e.to_string())?;
            if language != "auto" {
                form = form.text("language", language.to_string());
            }
            let result: Value = client
                .post(url.clone())
                .bearer_auth(&key)
                .multipart(form)
                .send()
                .map_err(|e| e.to_string())?
                .error_for_status()
                .map_err(|e| e.to_string())?
                .json()
                .map_err(|e| e.to_string())?;
            let text = result["text"]
                .as_str()
                .ok_or("The speech provider returned no transcript.")?
                .trim()
                .to_string();
            segments.push(
                json!({"start":offset,"end":offset+duration,"text":text,"speaker":"Microphone"}),
            );
            texts.push(text);
            offset += duration;
            let _ = app.emit(
                "progress",
                json!({"message":"Transcribing with your cloud provider","fraction":0}),
            );
        }
        Ok((texts.join(" "), Value::Array(segments)))
    })();
    let _ = std::fs::remove_file(chunk);
    result
}
