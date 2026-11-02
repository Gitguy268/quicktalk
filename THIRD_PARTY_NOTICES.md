# Third-party notices

QuickTalk’s original code is MIT licensed. Dependencies retain their own licenses. Lockfiles identify the concrete resolved versions.

- whisper.cpp / ggml — MIT, commit `231638a0717b7822784297e5980965ffcec05810`. See WHISPER-LICENSE. Source: https://github.com/ggml-org/whisper.cpp
- OpenAI Whisper model weights — MIT; distributed separately on request through the whisper.cpp model repository. Source: https://github.com/openai/whisper
- Tauri — MIT/Apache-2.0; Rust and frontend libraries retain their package licenses.
- Svelte, Vite, Lucide — MIT/ISC according to their package distributions. Icons are supplied by Lucide.
- SQLite — public domain; bundled through rusqlite.
- CPAL, Enigo, reqwest, hound, keyring, rfd and other Cargo dependencies — see Cargo.lock and upstream crate license metadata.
- Optional pyannote model and Python dependencies are installed separately; their own model access terms and licenses apply. They are not part of the default application bundle.

Handy, OpenWhispr, and the other supplied repositories were evaluated as references. This application is an original implementation, not a renamed fork. No proprietary Wispr Flow or Granola code, assets, credentials, or services are bundled. REA static research outputs remain outside the distributable source tree; findings and limitations are summarized in RESEARCH.md.

Before public distribution, audit transitive package notices for the exact release lockfiles and include required notices with installers.
