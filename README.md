# Mass Transcriptor (Tauri + SQLite)

Local desktop app for mass audio/video transcription. **No login, signup, or multi-user auth** — launching opens the workspace directly.

## Stack

- **Tauri 2** (Rust backend)
- **React + Vite** UI
- **SQLite** via `rusqlite` (bundled)
- **AssemblyAI** for transcription (API key in Settings)
- Local **Whisper (whisper.cpp)** via Settings; ffmpeg required on PATH

### Whisper (local)

1. Install `ffmpeg` and confirm `ffmpeg -version` works in a terminal.
2. Open Settings, set default provider to `whisper`, pick `tiny`, `base`, or `small` (default `base`).
3. No API key is required for Whisper.
4. The first job that uses a given model size downloads `ggml-{size}.bin` from Hugging Face into the app data directory (tiny ~75 MB, base ~142 MB, small ~466 MB). Later jobs reuse the file.
5. If ffmpeg is missing, the job fails with an install/retry message.

Building the desktop app compiles whisper.cpp (cmake + a C/C++ compiler required).

## Features

- Upload audio/video files → transcription jobs
- Multi-file uploads create a **batch**
- Job list / job detail / batch detail
- Settings: workspace name, default provider, language, AssemblyAI API key, Whisper model `tiny` | `base` | `small` (first job downloads ggml weights into app data `whisper-models/`)
- Retry failed jobs
- Download transcript markdown

## Security

- **Do not commit API keys.** Put your AssemblyAI key only in the app **Settings** UI (stored in local SQLite under the OS app data directory).
- ggml model files and API keys stay in the OS app data directory; do not commit them.
- Never commit `.env`, `*.db`, or `storage/` — they are gitignored.
- Tests use fake placeholder keys only (`test-key`, etc.), never production credentials.

## Develop

```bash
npm install
npm run tauri:dev
```

## Test

```bash
npm test
# or
cargo test --manifest-path src-tauri/Cargo.toml
```

## Data location

App data (SQLite DB + media + `whisper-models/`) lives under the OS app data directory for
`com.matheuspuppe.mass-transcriptor-tauri`.
