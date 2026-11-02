#[cfg(target_os = "linux")]
use std::io::Read;
use std::{
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::Duration,
};
pub enum Capture {
    Process(Child, Option<std::thread::JoinHandle<Result<(), String>>>),
    #[cfg(windows)]
    Native(crate::audio::Recording),
}
impl Capture {
    pub fn start(app: &tauri::AppHandle, path: PathBuf) -> Result<Self, String> {
        #[cfg(target_os = "macos")]
        {
            let mut child = Command::new(crate::speech::binary(app, "quicktalk-system-audio")?)
                .arg(&path)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| e.to_string())?;
            let stdout = child.stdout.take().unwrap();
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let mut line = String::new();
                let _ = BufReader::new(stdout).read_line(&mut line);
                let _ = tx.send(line);
            });
            match rx.recv_timeout(Duration::from_secs(20)) {
                Ok(s) if s.trim() == "READY" => Ok(Self::Process(child, None)),
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    Err("Computer audio could not start. Allow QuickTalk screen/system-audio recording in Privacy & Security, then restart the app.".into())
                }
            }
        }
        #[cfg(windows)]
        {
            crate::audio::start(path, "__system__".into(), app.clone(), "system".into())
                .map(Self::Native)
        }
        #[cfg(target_os = "linux")]
        {
            let _ = app;
            let mut child=Command::new("parec").args(["--device=@DEFAULT_MONITOR@","--raw","--rate=16000","--channels=1","--format=s16le"]).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().map_err(|_|"Install PulseAudio utilities (parec) to capture computer audio through PulseAudio or PipeWire-Pulse.")?;
            let mut stdout = child.stdout.take().unwrap();
            let worker = std::thread::spawn(move || {
                let mut writer = hound::WavWriter::create(
                    path,
                    hound::WavSpec {
                        channels: 1,
                        sample_rate: 16000,
                        bits_per_sample: 16,
                        sample_format: hound::SampleFormat::Int,
                    },
                )
                .map_err(|e| e.to_string())?;
                let mut b = [0u8; 2];
                let mut count = 0;
                while stdout.read_exact(&mut b).is_ok() {
                    writer
                        .write_sample(i16::from_le_bytes(b))
                        .map_err(|e| e.to_string())?;
                    count += 1;
                    if count % 32000 == 0 {
                        writer.flush().map_err(|e| e.to_string())?;
                    }
                }
                writer.finalize().map_err(|e| e.to_string())?;
                if count == 0 {
                    Err("No system audio was captured. Check your monitor source.".into())
                } else {
                    Ok(())
                }
            });
            Ok(Self::Process(child, Some(worker)))
        }
    }
    pub fn stop(self) -> Result<(), String> {
        match self {
            Self::Process(mut child, worker) => {
                #[cfg(target_os = "macos")]
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(b"stop\n");
                }
                #[cfg(not(target_os = "macos"))]
                let _ = child.kill();
                let until = std::time::Instant::now() + Duration::from_secs(10);
                loop {
                    if child.try_wait().map_err(|e| e.to_string())?.is_some() {
                        break;
                    }
                    if std::time::Instant::now() > until {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err("Computer audio did not stop cleanly.".into());
                    }
                    std::thread::sleep(Duration::from_millis(30));
                }
                if let Some(w) = worker {
                    w.join().map_err(|_| "System audio worker failed")??;
                }
                Ok(())
            }
            #[cfg(windows)]
            Self::Native(record) => {
                record
                    .stop
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                record
                    .handle
                    .join()
                    .map_err(|_| "System audio worker failed")??;
                Ok(())
            }
        }
    }
}
