# Validation — 2026-10-24

Environment: Apple Silicon macOS 27.0.1; Node 26.12.1; Rust 1.99.1; Apple command-line tools.

| Check | Result |
|---|---|
| Svelte / TypeScript | 0 errors, 0 warnings |
| Production frontend | Passed; approximately 113 KB JavaScript before compression |
| Rust tests | 5 passed: existing storage/snippet tests plus repository/path validation, verified model persistence/removal, corrupt/truncated/cancelled download cleanup |
| Optimized macOS bundle | Built successfully with Hugging Face importer; approximately 11.4 MiB |
| Portable CPU speech engine | Public JFK WAV transcribed correctly using tiny Q5_1, English, 4 threads |
| Speech-worker memory | 140,148,736 bytes maximum resident set size (~133.66 MiB); 134,988,448 bytes peak footprint |
| Speech sample time | 0.61 seconds wall time for the 10.58-second fixture; warm-cache single sample, not a general latency claim |
| UI visual check | Browser preview inspected; navigation and light/dark selection exercised |
| Native app launch / UI | Attempt blocked by missing Computer Use permissions; not claimed verified |
| Microphone / native insertion | Not exercised |
| macOS system-audio helper | Compiled for macOS 14 minimum; permission and capture tests outstanding |
| Windows / Linux | Source paths and CI matrix supplied; builds and hands-on tests outstanding |
| Calendar / cloud / optional speaker model | Implemented integration paths; live credentials/models not configured or tested |
| Two-hour recording / disk-full / crash tests | Outstanding |
| Total app idle-memory target | Not measured; do not claim achieved |

The initial Metal-enabled engine failed when tested in the restricted execution environment. The shipped engine uses a portable CPU-only build and passes the sample. No GPU dependency remains in the default build.

Test output is not proof of universal transcription accuracy. Broad multilingual and noisy/mixed-language accuracy evaluation remains to be done.

See README.md → Current boundaries for missing product features. Do not publish a stable release until platform checks and dependency notices are completed.

## Hugging Face import validation

A separate live integration test passed: queried `ggerganov/whisper.cpp`, pinned revision `5359861c739e955e79d9a303bcbc70fb988958b1`, downloaded `ggml-tiny-q5_1.bin` (32,152,673 bytes), verified the checksum and header, persisted its manifest, resolved it through the same model path used for dictation, and ran the bundled speech engine on the public JFK sample. The expected “ask not what your country can do for you” transcript was asserted. Temporary test model data was removed afterward.

The opt-in network test is ignored during normal offline tests. Run it with `cargo test --manifest-path src-tauri/Cargo.toml real_hugging_face_import -- --ignored --nocapture`; set `QUICKTALK_TEST_WAV` to the whisper.cpp JFK WAV fixture to include the engine transcription assertion.

The import panel was inspected in the browser preview. Native Tauri UI import clicks and gated/private repository credentials remain untested. Imports use the shared Rust backend across platforms; Windows and Linux builds have not been executed here. Arbitrary Hugging Face architectures are not supported by the bundled Whisper runtime.
