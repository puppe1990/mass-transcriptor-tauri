# Mass Transcriptor (Tauri + SQLite)

Local desktop app for mass audio/video transcription. **No login, signup, or multi-user auth** — launching opens the workspace directly.

## Stack

- **Tauri 2** (Rust backend)
- **React + Vite** UI
- **SQLite** via `rusqlite` (bundled)
- **AssemblyAI** for transcription (API key in Settings)

## Features

- Upload audio/video files → transcription jobs
- Multi-file uploads create a **batch**
- Job list / job detail / batch detail
- Settings: workspace name, default provider, language, AssemblyAI API key
- Retry failed jobs
- Download transcript markdown

## Security

- **Do not commit API keys.** Put your AssemblyAI key only in the app **Settings** UI (stored in local SQLite under the OS app data directory).
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

App data (SQLite DB + media) lives under the OS app data directory for
`com.matheuspuppe.mass-transcriptor-tauri`.
