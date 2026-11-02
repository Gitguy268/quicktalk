//! Hugging Face imports for the bundled whisper.cpp runtime.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct HubFile {
    pub name: String,
    pub size: u64,
    pub sha256: Option<String>,
    pub compatible: bool,
    pub reason: String,
}
#[derive(Serialize, Deserialize)]
pub struct Repository {
    pub repo: String,
    pub revision: String,
    pub license: String,
    pub files: Vec<HubFile>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ImportedModel {
    pub id: String,
    pub name: String,
    pub repo: String,
    pub revision: String,
    pub filename: String,
    pub sha256: String,
    pub size: u64,
}
fn valid_hash(s: &str, len: usize) -> bool {
    s.len() == len && s.bytes().all(|b| b.is_ascii_hexdigit())
}
pub fn repository_id(input: &str) -> Result<String, String> {
    let input = input.trim().trim_end_matches('/');
    let name = if input.starts_with("https://") {
        let url = reqwest::Url::parse(input).map_err(|_| "Invalid Hugging Face URL")?;
        if url.host_str() != Some("huggingface.co")
            || url.port().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err("Paste a Hugging Face model page, such as https://huggingface.co/ggerganov/whisper.cpp".into());
        }
        url.path().trim_matches('/').to_string()
    } else {
        input.to_string()
    };
    let parts: Vec<_> = name.split('/').collect();
    if parts.len() != 2
        || parts.iter().any(|p| {
            p.is_empty()
                || *p == "."
                || *p == ".."
                || !p
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
    {
        return Err(
            "Enter a model repository as owner/name, or paste its Hugging Face page URL.".into(),
        );
    }
    Ok(name)
}
fn url(parts: &[&str]) -> reqwest::Url {
    let mut url = reqwest::Url::parse("https://huggingface.co").unwrap();
    url.path_segments_mut()
        .unwrap()
        .extend(parts.iter().copied());
    url
}
fn request(
    client: &reqwest::blocking::Client,
    url: reqwest::Url,
    token: &str,
) -> reqwest::blocking::RequestBuilder {
    let r = client.get(url);
    if token.trim().is_empty() {
        r
    } else {
        r.bearer_auth(token.trim())
    }
}
fn response_error(e: reqwest::Error) -> String {
    match e.status().map(|s| s.as_u16()) {
        Some(401 | 403) => "Access denied. Add a Hugging Face read token and accept any model access terms on its page.".into(),
        Some(404) => "Repository or revision not found. Private repositories need a read token.".into(),
        Some(429) => "Hugging Face is rate limiting requests. Please try again shortly.".into(),
        _ => format!("Hugging Face request failed: {}", e.without_url()),
    }
}
pub fn inspect(input: &str, revision: &str, token: &str) -> Result<Repository, String> {
    let repo = repository_id(input)?;
    let revision = if revision.trim().is_empty() {
        "main"
    } else {
        revision.trim()
    };
    if revision.len() > 200 || revision.chars().any(char::is_control) {
        return Err("Invalid revision".into());
    }
    let (owner, name) = repo.split_once('/').unwrap();
    let mut endpoint = url(&["api", "models", owner, name, "revision", revision]);
    endpoint.query_pairs_mut().append_pair("blobs", "true");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(response_error)?;
    let response = request(&client, endpoint, token)
        .send()
        .map_err(response_error)?
        .error_for_status()
        .map_err(response_error)?;
    let mut bytes = Vec::new();
    response
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("Repository listing is too large.".into());
    }
    let data: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let commit = data["sha"]
        .as_str()
        .ok_or("Repository has no revision")?
        .to_string();
    if !valid_hash(&commit, 40) && !valid_hash(&commit, 64) {
        return Err("Repository returned an invalid revision".into());
    }
    let mut files: Vec<HubFile> = data["siblings"]
        .as_array()
        .ok_or("Repository has no file list")?
        .iter()
        .filter_map(|item| {
            let name = item["rfilename"].as_str()?;
            if ![".bin", ".gguf", ".safetensors", ".pt", ".onnx"]
                .iter()
                .any(|ext| name.ends_with(ext))
            {
                return None;
            }
            let sha256 = item["lfs"]["sha256"]
                .as_str()
                .filter(|s| valid_hash(s, 64))
                .map(str::to_string);
            let size = item["size"]
                .as_u64()
                .or_else(|| item["lfs"]["size"].as_u64())
                .unwrap_or(0);
            let candidate = name.ends_with(".bin")
                && !name
                    .split('/')
                    .last()
                    .unwrap_or("")
                    .starts_with("pytorch_model");
            let compatible =
                candidate && sha256.is_some() && size >= 48 && size <= 8 * 1024 * 1024 * 1024;
            let reason = if !candidate {
                "Needs a different runtime. Choose a whisper.cpp GGML .bin conversion."
            } else if sha256.is_none() {
                "No SHA-256 checksum is available for this file."
            } else if !compatible {
                "Outside the supported model size range (48 bytes–8 GB)."
            } else {
                "GGML candidate · Whisper format checked during download"
            };
            Some(HubFile {
                name: name.into(),
                size,
                sha256,
                compatible,
                reason: reason.into(),
            })
        })
        .collect();
    files.sort_by(|a, b| {
        b.compatible
            .cmp(&a.compatible)
            .then(a.size.cmp(&b.size))
            .then(a.name.cmp(&b.name))
    });
    Ok(Repository {
        repo,
        revision: commit,
        license: data["cardData"]["license"]
            .as_str()
            .unwrap_or("See model page")
            .to_string(),
        files,
    })
}
fn model_dir(root: &Path, id: &str) -> Result<PathBuf, String> {
    let hash = id.strip_prefix("hf-").ok_or("Unknown model")?;
    if !valid_hash(hash, 64) {
        return Err("Invalid imported model ID".into());
    }
    Ok(root.join("models").join(id))
}
pub fn model_path(root: &Path, id: &str) -> Result<PathBuf, String> {
    let dir = model_dir(root, id)?;
    let model: ImportedModel = serde_json::from_slice(
        &fs::read(dir.join("manifest.json")).map_err(|_| "Imported model is not installed")?,
    )
    .map_err(|e| e.to_string())?;
    if model.id != id {
        return Err("Imported model manifest is invalid".into());
    }
    Ok(dir.join("model.bin"))
}
pub fn installed(root: &Path) -> Result<Vec<ImportedModel>, String> {
    let mut models = Vec::new();
    let dir = root.join("models");
    if !dir.exists() {
        return Ok(models);
    }
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let id = entry.file_name().to_string_lossy().to_string();
        if !entry.file_type().map_err(|e| e.to_string())?.is_dir() || model_dir(root, &id).is_err()
        {
            continue;
        }
        if let Ok(data) = fs::read(entry.path().join("manifest.json")) {
            if let Ok(model) = serde_json::from_slice::<ImportedModel>(&data) {
                if model.id == id && entry.path().join("model.bin").is_file() {
                    models.push(model);
                }
            }
        }
    }
    models.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(models)
}
pub fn remove(root: &Path, id: &str) -> Result<(), String> {
    let dir = model_dir(root, id)?;
    if dir.exists() {
        fs::remove_dir_all(dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn whisper_header(header: &[u8]) -> bool {
    if header.len() < 48 {
        return false;
    }
    let n: Vec<i32> = header[..48]
        .chunks_exact(4)
        .map(|b| i32::from_le_bytes(b.try_into().unwrap()))
        .collect();
    n[0] == 0x67676d6c
        && (50000..=60000).contains(&n[1])
        && n[2] == 1500
        && (128..=2048).contains(&n[3])
        && (1..=32).contains(&n[4])
        && (1..=64).contains(&n[5])
        && n[6] == 448
        && n[7] == n[3]
        && (1..=32).contains(&n[8])
        && (1..=64).contains(&n[9])
        && [80, 128].contains(&n[10])
        && (0..=10000).contains(&n[11])
}
fn install_stream(
    root: &Path,
    model: &ImportedModel,
    mut response: impl Read,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64, u64),
) -> Result<(), String> {
    let dir = model_dir(root, &model.id)?;
    if dir.join("manifest.json").exists() && dir.join("model.bin").exists() {
        return Ok(());
    }
    fs::create_dir_all(root.join("models")).map_err(|e| e.to_string())?;
    let staging = root
        .join("models")
        .join(format!(".download-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&staging).map_err(|e| e.to_string())?;
    let result = (|| {
        let mut out = fs::File::create(staging.join("model.bin")).map_err(|e| e.to_string())?;
        let mut hash = Sha256::new();
        let mut received = 0u64;
        let mut header = Vec::new();
        let mut buf = [0u8; 65536];
        let mut last = std::time::Instant::now();
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err("Download cancelled".to_string());
            }
            let n = response.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            if header.len() < 48 {
                header.extend_from_slice(&buf[..n.min(48 - header.len())]);
            }
            if header.len() == 48 && !whisper_header(&header) {
                return Err("This file is not a supported Whisper GGML model. Choose a whisper.cpp conversion; PyTorch, Safetensors and GGUF need other runtimes.".into());
            }
            received += n as u64;
            if received > model.size {
                return Err("Model exceeds its advertised size".into());
            }
            hash.update(&buf[..n]);
            out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
            if last.elapsed() > Duration::from_millis(200) {
                progress(received, model.size);
                last = std::time::Instant::now();
            }
        }
        if received != model.size
            || !whisper_header(&header)
            || format!("{:x}", hash.finalize()) != model.sha256
        {
            return Err(
                "Model verification failed: size, format or checksum did not match. Please retry."
                    .into(),
            );
        }
        out.sync_all().map_err(|e| e.to_string())?;
        drop(out);
        fs::write(
            staging.join("manifest.json"),
            serde_json::to_vec_pretty(model).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        if dir.exists() {
            return Err(
                "An incomplete model folder already exists. Remove it before retrying.".into(),
            );
        }
        fs::rename(&staging, &dir).map_err(|e| e.to_string())?;
        progress(received, model.size);
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}
pub fn download(
    root: &Path,
    repo: &str,
    revision: &str,
    filename: &str,
    token: &str,
    cancel: &AtomicBool,
    progress: impl FnMut(u64, u64),
) -> Result<ImportedModel, String> {
    // Re-read metadata at the chosen commit; never accept a URL or checksum from the UI.
    let info = inspect(repo, revision, token)?;
    let file = info
        .files
        .iter()
        .find(|f| f.name == filename && f.compatible)
        .ok_or("Choose a compatible GGML .bin file from this repository")?;
    let id = format!(
        "hf-{:x}",
        Sha256::digest(format!("{}\n{}\n{}", info.repo, info.revision, file.name).as_bytes())
    );
    let model = ImportedModel {
        id,
        name: file
            .name
            .rsplit('/')
            .next()
            .unwrap_or(filename)
            .trim_end_matches(".bin")
            .to_string(),
        repo: info.repo.clone(),
        revision: info.revision.clone(),
        filename: file.name.clone(),
        sha256: file.sha256.clone().unwrap(),
        size: file.size,
    };
    if model_path(root, &model.id)
        .map(|p| p.is_file())
        .unwrap_or(false)
    {
        return Ok(model);
    }
    if cancel.load(Ordering::Relaxed) {
        return Err("Download cancelled".into());
    }
    let (owner, name) = info.repo.split_once('/').unwrap();
    let mut parts = vec![owner, name, "resolve", &info.revision];
    parts.extend(file.name.split('/'));
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(3600))
        .build()
        .map_err(response_error)?;
    let response = request(&client, url(&parts), token)
        .send()
        .map_err(response_error)?
        .error_for_status()
        .map_err(response_error)?;
    install_stream(root, &model, response, cancel, progress)?;
    Ok(model)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (PathBuf, ImportedModel, Vec<u8>) {
        let root = std::env::temp_dir().join(format!("quicktalk-hub-{}", uuid::Uuid::new_v4()));
        let mut data: Vec<u8> = [
            0x67676d6ci32,
            51865,
            1500,
            384,
            6,
            4,
            448,
            384,
            6,
            4,
            80,
            1009,
        ]
        .iter()
        .flat_map(|n| n.to_le_bytes())
        .collect();
        data.extend(vec![7; 100000]);
        let model = ImportedModel {
            id: format!("hf-{}", "a".repeat(64)),
            name: "Test".into(),
            repo: "test/model".into(),
            revision: "b".repeat(40),
            filename: "ggml-test.bin".into(),
            sha256: format!("{:x}", Sha256::digest(&data)),
            size: data.len() as u64,
        };
        (root, model, data)
    }
    #[test]
    fn rejects_foreign_urls_and_traversal() {
        assert_eq!(
            repository_id("https://huggingface.co/ggerganov/whisper.cpp/").unwrap(),
            "ggerganov/whisper.cpp"
        );
        for input in [
            "https://evil.example/a/b",
            "../model",
            "a/b/c",
            "https://huggingface.co/a/b?token=x",
            "https://user@huggingface.co/a/b",
        ] {
            assert!(repository_id(input).is_err(), "{input}");
        }
        assert!(model_dir(Path::new("/tmp"), "hf-../../escape").is_err());
    }
    #[test]
    fn verified_import_persists_and_can_be_removed() {
        let (root, model, data) = fixture();
        install_stream(&root, &model, &data[..], &AtomicBool::new(false), |_, _| {}).unwrap();
        assert_eq!(
            fs::read(model_path(&root, &model.id).unwrap()).unwrap(),
            data
        );
        assert_eq!(installed(&root).unwrap()[0].repo, "test/model");
        remove(&root, &model.id).unwrap();
        assert!(installed(&root).unwrap().is_empty());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn failed_or_cancelled_downloads_leave_no_installation() {
        for mode in 0..4 {
            let (root, mut model, mut data) = fixture();
            if mode == 0 {
                data[0] = 0;
            }
            if mode == 1 {
                model.sha256 = "0".repeat(64);
            }
            if mode == 2 {
                data.truncate(70);
            }
            assert!(install_stream(
                &root,
                &model,
                &data[..],
                &AtomicBool::new(mode == 3),
                |_, _| {}
            )
            .is_err());
            assert!(installed(&root).unwrap().is_empty());
            assert_eq!(fs::read_dir(root.join("models")).unwrap().count(), 0);
            fs::remove_dir_all(root).unwrap();
        }
    }
    #[test]
    #[ignore = "downloads a real 32 MB model from Hugging Face"]
    fn real_hugging_face_import() {
        let root = std::env::temp_dir().join(format!("quicktalk-hf-live-{}", uuid::Uuid::new_v4()));
        let info = inspect("ggerganov/whisper.cpp", "main", "").unwrap();
        let model = download(
            &root,
            &info.repo,
            &info.revision,
            "ggml-tiny-q5_1.bin",
            "",
            &AtomicBool::new(false),
            |_, _| {},
        )
        .unwrap();
        assert!(model_path(&root, &model.id).unwrap().is_file());
        assert_eq!(
            crate::speech::model_path(&root, &model.id).unwrap(),
            model_path(&root, &model.id).unwrap()
        );
        if let Ok(sample) = std::env::var("QUICKTALK_TEST_WAV") {
            let engine =
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("binaries")
                    .join(if cfg!(windows) {
                        "whisper-cli.exe"
                    } else {
                        "whisper-cli"
                    });
            let output = std::process::Command::new(engine)
                .arg("-m")
                .arg(model_path(&root, &model.id).unwrap())
                .arg("-f")
                .arg(sample)
                .args(["-l", "en", "-np", "-ng", "-t", "4"])
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let text = String::from_utf8_lossy(&output.stdout);
            assert!(
                text.contains("ask not what your country can do for you"),
                "Unexpected transcript: {text}"
            );
            println!("Imported model transcription: {text}");
        }
        println!(
            "Verified live model: {}/{} at {}",
            model.repo, model.filename, model.revision
        );
        fs::remove_dir_all(root).unwrap();
    }
}
