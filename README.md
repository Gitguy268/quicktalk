# QuickTalk

A quiet place for your voice. Local-first desktop dictation and meeting notes, built with Rust, Tauri 2, and Svelte 5.

<!-- live-links -->
**Project page:** [gitguy268.github.io/quicktalk](https://gitguy268.github.io/quicktalk/) · [quicktalk-pied.vercel.app](https://quicktalk-pied.vercel.app)
<!-- /live-links -->

<!-- run-steps:start -->
## Run it locally, step by step

1. **Install** [Node.js](https://nodejs.org) 22 or newer, current stable [Rust](https://rustup.rs), Python 3, Git, CMake and a C++ toolchain. macOS also needs Apple's command line tools.

2. **Get the code:**

   ```sh
   git clone https://github.com/Gitguy268/quicktalk.git
   cd quicktalk
   ```

3. **Install the dependencies and build the speech engine:**

   ```sh
   npm ci
   python3 scripts/build-engine.py
   ```

4. **Start it:**

   ```sh
   npm run tauri -- dev
   ```

5. **In the app,** choose Settings → Speech models → Everyday → Download, then press `Cmd/Ctrl+Shift+Space` to dictate.
<!-- run-steps:end -->

## Install

**macOS (Homebrew):**

```sh
brew trust gitguy268/tap
brew install --cask gitguy268/tap/quicktalk
```

The cask installs the release build (Apple silicon or Intel, macOS 14+). It isn't notarised, so the cask clears the quarantine flag.

<details>
<summary><b>Linux (Flatpak)</b></summary>

Download `io.github.gitguy268.quicktalk.flatpak` from the [latest release](https://github.com/Gitguy268/quicktalk/releases/latest), then:

```sh
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user ./io.github.gitguy268.quicktalk.flatpak
flatpak run io.github.gitguy268.quicktalk
```

This Flatpak is built by this repo's workflow; it isn't on Flathub. [packaging/flatpak](packaging/flatpak/README.md) has the build steps.

</details>

## Start

Open the QuickTalk app, then choose **Settings → Speech models → Everyday → Download**. Downloading is a one-time operation. Choose **Light** for the smallest model. You can record before downloading; audio is retained if transcription cannot run.

Press **Command/Ctrl + Shift + Space** to start and stop dictation. The shortcut can be changed, including hold-to-talk behavior. On macOS, allow microphone and Accessibility access when prompted. Start from the shortcut with your cursor already in the destination app. QuickTalk only inserts if the original app is still active; the transcript is always saved.

**Meetings** records the microphone. Enable **Include computer audio** in General settings to capture the other side separately. Use headphones to reduce duplicated audio. Transcription runs after you finish; this release does not provide a live meeting transcript. Windows uses WASAPI loopback; macOS uses ScreenCaptureKit; Linux needs `parec` from PulseAudio utilities, also usable with PipeWire-Pulse.

Import accepts **16-bit PCM WAV** audio. Export supports Markdown (including Obsidian), text, SRT, and JSON. A recording’s JSON includes its timestamped original segments. Edited transcript text does not rewrite original segment timing.

## Hugging Face models

Open **Settings → Speech models → From Hugging Face**. Paste a model repository URL or `owner/name`, choose **Find model files**, select a Whisper GGML `.bin` file, then **Download & use**. The imported model is selected for both dictation and meetings and stays available offline after restarting QuickTalk. Select another model before removing an active import.

You can browse any model repository. **The bundled runtime runs Whisper models converted for whisper.cpp, not every architecture on Hugging Face.** PyTorch, Safetensors, ONNX, and GGUF models are shown as unsupported. For example, the original `openai/whisper-large-v3-turbo` repository needs a whisper.cpp conversion; use `ggerganov/whisper.cpp` and choose a large-v3-turbo GGML file. A `.bin` extension alone does not guarantee compatibility: the importer checks the Whisper header and the speech engine performs full loading at transcription time. Custom tokenizers/architectures that need repository code are not supported.

The optional revision field accepts a branch, tag, or commit; QuickTalk resolves it to an immutable commit for the download. Private/gated access can use a Hugging Face read token entered in the expanded options. That token remains in memory for the import and is cleared after success; it is not saved in settings or the model manifest. Model access terms must be accepted on Hugging Face first. Check the model's license before using or redistributing it.

Downloads stream to disk with progress and cancellation. SHA-256, exact size, and Whisper header are verified before the model becomes selectable. Failed/cancelled downloads are removed. Models are stored under `models/hf-<id>/` in QuickTalk's app-data directory, with source repository, pinned revision, and checksum recorded in a manifest. No Python runtime or model code is downloaded. Model size is shown before download; RAM during inference depends on the selected model. Private repository access is implemented but has not been tested with account credentials.

## Optional AI

For private summaries and meeting questions, install Ollama and download a model (for example `ollama pull qwen3:4b`), then select it under AI & privacy. QuickTalk requests model unloading after a response. Local language models can use several GB independently of QuickTalk’s speech model.

A compatible remote chat-completions service can be used instead. Enter its base URL including `/v1`, model name, and API key. Only text explicitly submitted for an AI action is sent. Cloud speech is a separate opt-in: enter the full HTTPS transcription endpoint, model, and speech key. Audio is uploaded in bounded chunks. There is no automatic cloud fallback.

Keys and OAuth tokens use the OS credential store. Local transcripts are in SQLite in the OS app-data directory; they are not encrypted by QuickTalk. No telemetry or QuickTalk account is included.

## Calendars

This independent open-source build does not ship someone else’s OAuth credentials. Register your own desktop OAuth application, enter its client ID under Calendars, and connect through your browser.

- **Google:** enable Calendar API, configure a consent screen and your test user, create an OAuth Desktop client, and supply its client ID and desktop client secret. Uses read-only event scope and a temporary loopback redirect.
- **Microsoft:** register a public desktop application supporting the intended account types, enable the loopback redirect `http://127.0.0.1`, and delegated `Calendars.Read` plus offline access. Microsoft app registration may require explicitly allowing the localhost redirect URI used by your registration.
- Refresh displays up to 100 events over the next 14 days. Recurring instances are expanded by the providers. Clicking an event explicitly starts recording. No background calendar monitoring or automatic recording is enabled.
- Calendar connections need live account testing with your registrations. This build does not claim verified OAuth access before that test.

## Optional speaker clustering

Run `python3 scripts/setup-speakers.py` with Python 3.11 or 3.12. The optional environment is intentionally outside the main app. It installs PyTorch/pyannote and may consume substantial disk space and RAM. Accept the Hugging Face access terms for `pyannote/speaker-diarization-community-1`, then enter a read token and the script’s printed Python path under AI & privacy.

Enable **Keep original audio** before recording. Open the meeting transcript and choose **Analyze speakers**. The process assigns anonymous voice clusters; it does not identify real people. Mixed computer audio is analyzed separately from the microphone. Overlapping speech and short turns can be inaccurate. This optional path requires real model-access testing; the default app does not install or load Python.

## Build

Requirements: Node.js 22+, current stable Rust, Python 3, Git, CMake, and a C++ toolchain. macOS needs Apple command-line tools; Windows needs MSVC build tools and WebView2; Linux needs WebKitGTK 4.1, GTK, ALSA, libxkbcommon, libdbus, and AppIndicator development packages (see CI).

```sh
npm ci
python3 scripts/build-engine.py
npm run check
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri -- dev
npm run tauri -- build
```

The engine is pinned to commit `231638a0717b7822784297e5980965ffcec05810`. The portable build uses CPU inference, four worker threads, and separate processes that exit after each chunk. No GPU is required. Model files remain outside the app bundle.

Build automation covers Apple Silicon, Intel macOS, Windows x64, and Linux x64. CI configuration is not evidence of a completed Windows/Linux test run. Test actual microphone, focus, shortcuts, system audio, and insertion on each OS before publishing a stable release. Signing/notarization requires your credentials.

## Linux desktop notes

On X11, install `xdotool` for target-window detection. On Wayland, configure a desktop shortcut invoking `quicktalk --toggle-dictation`; global keyboard hooks and direct text insertion are not portable across compositors. The initial build retains the transcript for manual copy on Wayland. The floating recorder is disabled on Linux to avoid focus theft. Wayland auto-insertion remains an explicit release gap.

## Resource behavior

Audio capture uses a bounded queue and writes to disk. Transcription reads 30-second chunks and launches an isolated model worker for each chunk; this limits memory at the cost of repeated model startup. Models are unloaded immediately after processing. Silence is skipped. Windows hidden in the tray currently retain their webview, so the idle-memory target must be measured with that included.

Targets: under 150 MB idle and approximately 1 GB during compact-model transcription. These are targets, not guaranteed results. Large speech models, speaker analysis, and local language models exceed the compact-mode budget. See VALIDATION.md for measured results and remaining checks.

## Current boundaries

This is a 0.1 development build, not a claim of complete Wispr Flow/Granola parity. Remaining work includes live meeting transcription, pause/resume, automatic calendar reminders, ICS import, per-app automatic style selection, editable speaker names, and full Wayland insertion. Summaries cite numbered excerpts rather than clickable timestamp links. Optional provider integrations need your credentials and end-to-end verification.

## Contributing

Run the frontend check, production build, and Rust tests before submitting changes. Keep recording/transcription work off the UI thread, never silently upload local audio, and add regression coverage for data-loss or concurrency fixes. Do not put API keys, model files, recordings, or proprietary research output into commits.

Original QuickTalk code is MIT licensed. See THIRD_PARTY_NOTICES.md for dependencies and research boundaries.
