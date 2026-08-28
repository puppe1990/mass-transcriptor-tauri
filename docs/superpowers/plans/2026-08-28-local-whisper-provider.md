# Local Whisper Provider Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `whisper` a working local transcription provider (whisper.cpp in-process, Settings model `tiny|base|small`, ggml download on first job, ffmpeg on PATH).

**Architecture:** Keep the existing prepare (short SQLite lock) → execute (no lock) → finish (short lock) worker. `PreparedTranscription` becomes an enum (`AssemblyAi` | `Whisper`). New domain modules own ffmpeg conversion, ggml download/verify, and a `WhisperEngine` trait; production uses `whisper-rs` + `std::process::Command` + reqwest. Tests inject fakes so CI never hits network, ffmpeg, or ggml files.

**Tech Stack:** Rust / Tauri 2, rusqlite, reqwest, sha1, whisper-rs 0.16, hound, ffmpeg CLI, React Settings UI.

**Spec:** `docs/superpowers/specs/2026-08-28-local-whisper-provider-design.md`

---

## File structure

Create:

- `src-tauri/src/domain/ffmpeg.rs` — `Ffmpeg` trait + `SystemFfmpeg`
- `src-tauri/src/domain/whisper_models.rs` — `WhisperModel`, `ModelDownloader`, `ensure_model`
- `src-tauri/src/domain/whisper.rs` — `WhisperEngine` trait, options, user-facing error strings, `WhisperRsEngine`

Modify:

- `src-tauri/src/domain/mod.rs` — declare new modules
- `src-tauri/src/domain/models.rs` — `whisper_model` on settings structs
- `src-tauri/src/db.rs` — column + guarded ALTER
- `src-tauri/src/domain/settings.rs` — persist/validate `whisper_model`
- `src-tauri/src/domain/jobs.rs` — enum prepare/execute, `TranscriptionDeps`
- `src-tauri/src/app_state.rs` — `models_dir`
- `src-tauri/src/lib.rs` — production deps in worker
- `src-tauri/Cargo.toml` — `sha1`, later `whisper-rs` + `hound`
- `src/lib/api.ts` — `whisperModel`
- `src/pages/SettingsPage.tsx` — enable Whisper + model select
- `src/pages/UploadPage.tsx` — provider-agnostic copy
- `README.md` — Whisper + ffmpeg

Do not restructure `jobs.rs` into new files; add private `execute_whisper` there.

---

### Task 1: Persist `whisper_model` in settings

**Files:**
- Modify: `src-tauri/src/domain/models.rs`
- Modify: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/domain/settings.rs`
- Modify: every `UpdateSettingsInput { ... }` literal in `settings.rs` and `jobs.rs` (add `whisper_model: "base".into()`)

- [ ] **Step 1: Write the failing tests**

Add to `src-tauri/src/domain/settings.rs` in `mod tests`:

```rust
#[test]
fn persists_whisper_provider_and_model() {
    let dir = tempdir().unwrap();
    let conn = db::open(&dir.path().join("s.db")).unwrap();

    let updated = update_settings(
        &conn,
        UpdateSettingsInput {
            workspace_name: "Studio".into(),
            default_provider: "whisper".into(),
            assemblyai_api_key: None,
            language: "pt".into(),
            whisper_model: "small".into(),
        },
    )
    .unwrap();
    assert_eq!(updated.default_provider, "whisper");
    assert_eq!(updated.whisper_model, "small");
    assert_eq!(get_settings(&conn).unwrap().whisper_model, "small");
}

#[test]
fn rejects_invalid_whisper_model() {
    let dir = tempdir().unwrap();
    let conn = db::open(&dir.path().join("s.db")).unwrap();
    let err = update_settings(
        &conn,
        UpdateSettingsInput {
            workspace_name: "Ok".into(),
            default_provider: "whisper".into(),
            assemblyai_api_key: None,
            language: "auto".into(),
            whisper_model: "medium".into(),
        },
    )
    .unwrap_err();
    assert!(err.contains("Whisper model"));
    assert!(err.contains("medium"));
}

#[test]
fn default_whisper_model_is_base() {
    let dir = tempdir().unwrap();
    let conn = db::open(&dir.path().join("s.db")).unwrap();
    assert_eq!(get_settings(&conn).unwrap().whisper_model, "base");
}

#[test]
fn switching_to_whisper_keeps_existing_api_key() {
    let dir = tempdir().unwrap();
    let conn = db::open(&dir.path().join("s.db")).unwrap();
    update_settings(
        &conn,
        UpdateSettingsInput {
            workspace_name: "A".into(),
            default_provider: "assemblyai".into(),
            assemblyai_api_key: Some("fixture-key-not-real".into()),
            language: "en".into(),
            whisper_model: "base".into(),
        },
    )
    .unwrap();
    let updated = update_settings(
        &conn,
        UpdateSettingsInput {
            workspace_name: "A".into(),
            default_provider: "whisper".into(),
            assemblyai_api_key: None,
            language: "en".into(),
            whisper_model: "tiny".into(),
        },
    )
    .unwrap();
    assert_eq!(
        updated.assemblyai_api_key.as_deref(),
        Some("fixture-key-not-real")
    );
    assert_eq!(updated.default_provider, "whisper");
    assert_eq!(updated.whisper_model, "tiny");
}
```

Also add `whisper_model: "base".into()` to existing `UpdateSettingsInput` literals in this file so they still compile once the field exists.

Add to `src-tauri/src/db.rs` tests:

```rust
let cols: Vec<String> = conn
    .prepare("PRAGMA table_info(app_settings)")
    .unwrap()
    .query_map([], |r| r.get::<_, String>(1))
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap();
assert!(cols.contains(&"whisper_model".into()));
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml settings:: -- --nocapture`

Expected: FAIL (unknown field `whisper_model` and/or missing struct field).

- [ ] **Step 3: Minimal implementation**

In `models.rs` add `whisper_model: String` to `AppSettings`. On `UpdateSettingsInput`:

```rust
fn default_whisper_model() -> String {
    "base".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsInput {
    pub workspace_name: String,
    pub default_provider: String,
    #[serde(default)]
    pub assemblyai_api_key: Option<String>,
    pub language: String,
    #[serde(default = "default_whisper_model")]
    pub whisper_model: String,
}
```

In `db.rs` `CREATE TABLE app_settings` add `whisper_model TEXT NOT NULL DEFAULT 'base'`. After the batch, migrate existing DBs:

```rust
let mut stmt = conn
    .prepare("PRAGMA table_info(app_settings)")
    .map_err(|e| e.to_string())?;
let names: Vec<String> = stmt
    .query_map([], |r| r.get::<_, String>(1))
    .map_err(|e| e.to_string())?
    .collect::<Result<_, _>>()
    .map_err(|e| e.to_string())?;
drop(stmt);
if !names.iter().any(|n| n == "whisper_model") {
    conn.execute(
        "ALTER TABLE app_settings ADD COLUMN whisper_model TEXT NOT NULL DEFAULT 'base'",
        [],
    )
    .map_err(|e| e.to_string())?;
}
```

In `settings.rs`:

- `const ALLOWED_WHISPER_MODELS: &[&str] = &["tiny", "base", "small"];`
- SELECT adds `whisper_model` (index 4)
- After language validation:

```rust
let whisper_model = input.whisper_model.trim().to_lowercase();
if !ALLOWED_WHISPER_MODELS.contains(&whisper_model.as_str()) {
    return Err(format!("Whisper model '{whisper_model}' is not allowed"));
}
```

- UPDATE sets `whisper_model = ?5`, `updated_at = ?6`

Add `whisper_model: "base".into()` to every `UpdateSettingsInput` in `jobs.rs` tests (`setup` plus the three later `update_settings` calls).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS (all existing + new settings/db tests).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/domain/models.rs src-tauri/src/db.rs src-tauri/src/domain/settings.rs src-tauri/src/domain/jobs.rs
git commit -m "feat: persist whisper_model in workspace settings"
```

---

### Task 2: Ffmpeg trait

**Files:**
- Create: `src-tauri/src/domain/ffmpeg.rs`
- Modify: `src-tauri/src/domain/mod.rs` — `pub mod ffmpeg;`

- [ ] **Step 1: Write the failing tests**

Create `ffmpeg.rs` with tests first (module can compile with empty impls failing):

```rust
//! Convert source media to 16 kHz mono WAV for whisper.cpp.

use std::path::Path;
use std::process::{Command, Stdio};

pub trait Ffmpeg: Send + Sync {
    fn is_available(&self) -> bool;
    fn to_wav_16k_mono(&self, src: &Path, dest: &Path) -> Result<(), String>;
}

pub struct SystemFfmpeg;

impl Ffmpeg for SystemFfmpeg {
    fn is_available(&self) -> bool {
        Command::new("ffmpeg")
            .arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn to_wav_16k_mono(&self, src: &Path, dest: &Path) -> Result<(), String> {
        let output = Command::new("ffmpeg")
            .args(["-y", "-i"])
            .arg(src)
            .args(["-ac", "1", "-ar", "16000", "-c:a", "pcm_s16le"])
            .arg(dest)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| e.to_string())?;
        if output.status.success() {
            Ok(())
        } else {
            Err("ffmpeg conversion failed".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct FlagFfmpeg {
        available: bool,
        convert_ok: bool,
    }

    impl Ffmpeg for FlagFfmpeg {
        fn is_available(&self) -> bool {
            self.available
        }
        fn to_wav_16k_mono(&self, _src: &Path, dest: &Path) -> Result<(), String> {
            if self.convert_ok {
                std::fs::write(dest, b"RIFF").map_err(|e| e.to_string())
            } else {
                Err("ffmpeg conversion failed".into())
            }
        }
    }

    #[test]
    fn reports_availability() {
        assert!(FlagFfmpeg {
            available: true,
            convert_ok: true
        }
        .is_available());
        assert!(!FlagFfmpeg {
            available: false,
            convert_ok: true
        }
        .is_available());
    }

    #[test]
    fn convert_writes_dest_or_errors() {
        let dest = PathBuf::from("not-used-until-ok");
        let err = FlagFfmpeg {
            available: true,
            convert_ok: false,
        }
        .to_wav_16k_mono(Path::new("x"), &dest)
        .unwrap_err();
        assert_eq!(err, "ffmpeg conversion failed");
    }
}
```

The production `SystemFfmpeg` is allowed in this file; tests must not call it.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml ffmpeg:: -- --nocapture`

Expected: FAIL until `mod ffmpeg` is declared (`could not find ffmpeg` or similar).

- [ ] **Step 3: Wire the module**

`src-tauri/src/domain/mod.rs`:

```rust
pub mod assemblyai;
pub mod ffmpeg;
pub mod grouping;
pub mod jobs;
pub mod markdown;
pub mod models;
pub mod settings;
pub mod storage;
```

Keep `ffmpeg.rs` as in Step 1.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml ffmpeg::`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/domain/ffmpeg.rs src-tauri/src/domain/mod.rs
git commit -m "feat: add ffmpeg conversion trait"
```

---

### Task 3: Ggml model ensure + SHA1

**Files:**
- Create: `src-tauri/src/domain/whisper_models.rs`
- Modify: `src-tauri/src/domain/mod.rs` — `pub mod whisper_models;`
- Modify: `src-tauri/Cargo.toml` — `sha1 = "0.10"`

- [ ] **Step 1: Write the failing tests**

Add `sha1 = "0.10"` to `[dependencies]` in `Cargo.toml`.

Create `whisper_models.rs` with types + tests (implementation of `ensure_artifact` can be stubbed to `unimplemented!` first if you prefer a red test; include full tests below).

```rust
//! Download and verify ggml Whisper weights.

use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhisperModel {
    Tiny,
    Base,
    Small,
}

impl WhisperModel {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.trim().to_lowercase().as_str() {
            "tiny" => Ok(Self::Tiny),
            "base" => Ok(Self::Base),
            "small" => Ok(Self::Small),
            other => Err(format!("Whisper model '{other}' is not allowed")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tiny => "tiny",
            Self::Base => "base",
            Self::Small => "small",
        }
    }

    pub fn filename(self) -> &'static str {
        match self {
            Self::Tiny => "ggml-tiny.bin",
            Self::Base => "ggml-base.bin",
            Self::Small => "ggml-small.bin",
        }
    }

    pub fn url(self) -> String {
        format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}",
            self.filename()
        )
    }

    pub fn sha1_hex(self) -> &'static str {
        match self {
            Self::Tiny => "bd577a113a864445d4c299885e0cb97d4ba92b5f",
            Self::Base => "465707469ff3a37a2b9b8d8f89f2f99de7299dac",
            Self::Small => "55356645c2b361a969dfd0ef2c5a50d530afd8d5",
        }
    }

    pub fn size_label(self) -> &'static str {
        match self {
            Self::Tiny => "~75 MB",
            Self::Base => "~142 MB",
            Self::Small => "~466 MB",
        }
    }
}

pub trait ModelDownloader: Send + Sync {
    fn download(&self, url: &str, dest: &Path) -> Result<(), String>;
}

pub fn sha1_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha1::digest(bytes))
}

pub fn ensure_model(
    models_dir: &Path,
    model: WhisperModel,
    downloader: &dyn ModelDownloader,
) -> Result<PathBuf, String> {
    ensure_artifact(
        models_dir,
        model.filename(),
        &model.url(),
        model.sha1_hex(),
        downloader,
    )
}

pub fn ensure_artifact(
    models_dir: &Path,
    filename: &str,
    url: &str,
    expected_sha1: &str,
    downloader: &dyn ModelDownloader,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(models_dir).map_err(|e| e.to_string())?;
    let target = models_dir.join(filename);
    if target.exists() {
        let bytes = std::fs::read(&target).map_err(|e| e.to_string())?;
        if sha1_hex(&bytes) == expected_sha1 {
            return Ok(target);
        }
        std::fs::remove_file(&target).map_err(|e| e.to_string())?;
    }

    let partial = models_dir.join(format!("{filename}.partial"));
    let _ = std::fs::remove_file(&partial);
    let download_result = downloader.download(url, &partial);
    if let Err(e) = download_result {
        let _ = std::fs::remove_file(&partial);
        return Err(e);
    }
    let bytes = match std::fs::read(&partial) {
        Ok(b) => b,
        Err(e) => {
            let _ = std::fs::remove_file(&partial);
            return Err(e.to_string());
        }
    };
    if sha1_hex(&bytes) != expected_sha1 {
        let _ = std::fs::remove_file(&partial);
        return Err("model sha1 mismatch".into());
    }
    std::fs::rename(&partial, &target).map_err(|e| e.to_string())?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::tempdir;

    struct CountingDownloader {
        payload: Vec<u8>,
        calls: AtomicUsize,
        fail: bool,
    }

    impl ModelDownloader for CountingDownloader {
        fn download(&self, _url: &str, dest: &Path) -> Result<(), String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                return Err("network down".into());
            }
            std::fs::write(dest, &self.payload).map_err(|e| e.to_string())
        }
    }

    #[test]
    fn parse_and_pins() {
        assert_eq!(WhisperModel::parse("BASE").unwrap(), WhisperModel::Base);
        assert!(WhisperModel::parse("medium").unwrap_err().contains("medium"));
        assert_eq!(WhisperModel::Tiny.filename(), "ggml-tiny.bin");
        assert_eq!(
            WhisperModel::Tiny.sha1_hex(),
            "bd577a113a864445d4c299885e0cb97d4ba92b5f"
        );
    }

    #[test]
    fn existing_valid_file_skips_download() {
        let dir = tempdir().unwrap();
        let payload = b"hello-model";
        let expected = sha1_hex(payload);
        std::fs::write(dir.path().join("ggml-base.bin"), payload).unwrap();
        let dl = CountingDownloader {
            payload: payload.to_vec(),
            calls: AtomicUsize::new(0),
            fail: false,
        };
        let path = ensure_artifact(dir.path(), "ggml-base.bin", "http://x", &expected, &dl).unwrap();
        assert_eq!(dl.calls.load(Ordering::SeqCst), 0);
        assert!(path.ends_with("ggml-base.bin"));
    }

    #[test]
    fn missing_file_downloads_and_renames() {
        let dir = tempdir().unwrap();
        let payload = b"hello-model";
        let expected = sha1_hex(payload);
        let dl = CountingDownloader {
            payload: payload.to_vec(),
            calls: AtomicUsize::new(0),
            fail: false,
        };
        let path = ensure_artifact(dir.path(), "ggml-tiny.bin", "http://x", &expected, &dl).unwrap();
        assert_eq!(dl.calls.load(Ordering::SeqCst), 1);
        assert!(path.exists());
        assert!(!dir.path().join("ggml-tiny.bin.partial").exists());
        assert_eq!(std::fs::read(&path).unwrap(), payload);
    }

    #[test]
    fn download_error_leaves_no_partial() {
        let dir = tempdir().unwrap();
        let dl = CountingDownloader {
            payload: b"x".to_vec(),
            calls: AtomicUsize::new(0),
            fail: true,
        };
        let err = ensure_artifact(dir.path(), "ggml-small.bin", "http://x", "deadbeef", &dl)
            .unwrap_err();
        assert_eq!(err, "network down");
        assert!(!dir.path().join("ggml-small.bin").exists());
        assert!(!dir.path().join("ggml-small.bin.partial").exists());
    }

    #[test]
    fn corrupt_existing_is_redownloaded() {
        let dir = tempdir().unwrap();
        let good = b"good-bytes";
        let expected = sha1_hex(good);
        std::fs::write(dir.path().join("ggml-base.bin"), b"corrupt").unwrap();
        let dl = CountingDownloader {
            payload: good.to_vec(),
            calls: AtomicUsize::new(0),
            fail: false,
        };
        let path = ensure_artifact(dir.path(), "ggml-base.bin", "http://x", &expected, &dl).unwrap();
        assert_eq!(dl.calls.load(Ordering::SeqCst), 1);
        assert_eq!(std::fs::read(path).unwrap(), good);
    }

    #[test]
    fn sha1_mismatch_after_download_deletes_partial() {
        let dir = tempdir().unwrap();
        let dl = CountingDownloader {
            payload: b"not-the-hash".to_vec(),
            calls: AtomicUsize::new(0),
            fail: false,
        };
        let err = ensure_artifact(dir.path(), "ggml-base.bin", "http://x", "00", &dl).unwrap_err();
        assert!(err.contains("sha1"));
        assert!(!dir.path().join("ggml-base.bin").exists());
        assert!(!dir.path().join("ggml-base.bin.partial").exists());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml whisper_models:: -- --nocapture`

Expected: FAIL until `mod whisper_models` is declared (or PASS if you added the full file + mod together — in that case skip to commit only after tests pass; if you add tests first without `ensure_artifact`, they must fail).

If you add the complete file and module in one edit, run the tests once: they must PASS before commit. Prefer: add `pub mod whisper_models;` then the file.

- [ ] **Step 3: Wire module**

`mod.rs`: `pub mod whisper_models;`

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/domain/whisper_models.rs src-tauri/src/domain/mod.rs
git commit -m "feat: download and verify ggml Whisper models"
```

---

### Task 4: WhisperEngine trait and error copy

**Files:**
- Create: `src-tauri/src/domain/whisper.rs`
- Modify: `src-tauri/src/domain/mod.rs` — `pub mod whisper;`

- [ ] **Step 1: Write the failing tests**

```rust
//! Local Whisper transcription (engine injected; production impl is whisper-rs).

use crate::domain::models::TranscriptionOutcome;
use crate::domain::whisper_models::WhisperModel;
use std::path::{Path, PathBuf};

pub const ERR_FFMPEG_MISSING: &str = "Whisper needs ffmpeg installed on this machine. Install ffmpeg, confirm the `ffmpeg` command works in a terminal, and retry this job.";

pub fn err_download(model: WhisperModel) -> String {
    format!(
        "Could not download Whisper model `{}` ({}) from Hugging Face. Check the network and retry. Incomplete files are not kept.",
        model.as_str(),
        model.size_label()
    )
}

pub fn err_convert(filename: &str) -> String {
    format!(
        "ffmpeg could not convert `{filename}` to 16 kHz WAV. Check that the file plays in a media player and that ffmpeg is up to date, then retry."
    )
}

pub fn err_infer(model: WhisperModel) -> String {
    format!(
        "Whisper transcription (`{}`) failed for this file. Retry the job; if it keeps failing, switch to `tiny` or use AssemblyAI.",
        model.as_str()
    )
}

#[derive(Debug, Clone)]
pub struct WhisperTranscribeOptions {
    pub model_path: PathBuf,
    pub language: Option<String>,
    pub model: WhisperModel,
}

pub trait WhisperEngine: Send + Sync {
    fn transcribe(
        &self,
        wav_path: &Path,
        opts: &WhisperTranscribeOptions,
    ) -> Result<TranscriptionOutcome, String>;
}

#[cfg(test)]
pub struct ScriptedWhisper {
    pub text: String,
    pub fail: bool,
}

#[cfg(test)]
impl WhisperEngine for ScriptedWhisper {
    fn transcribe(
        &self,
        _wav_path: &Path,
        opts: &WhisperTranscribeOptions,
    ) -> Result<TranscriptionOutcome, String> {
        if self.fail {
            return Err("engine boom".into());
        }
        Ok(TranscriptionOutcome {
            text: self.text.clone(),
            metadata: serde_json::json!({
                "model": opts.model.as_str(),
                "language": opts.language,
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_copy_matches_spec() {
        assert!(ERR_FFMPEG_MISSING.contains("ffmpeg"));
        let d = err_download(WhisperModel::Base);
        assert!(d.contains("`base`"));
        assert!(d.contains("~142 MB"));
        assert!(err_convert("clip.ogg").contains("`clip.ogg`"));
        assert!(err_infer(WhisperModel::Tiny).contains("`tiny`"));
    }

    #[test]
    fn scripted_engine_returns_text() {
        let engine = ScriptedWhisper {
            text: "hello from whisper".into(),
            fail: false,
        };
        let out = engine
            .transcribe(
                Path::new("x.wav"),
                &WhisperTranscribeOptions {
                    model_path: PathBuf::from("ggml-base.bin"),
                    language: Some("pt".into()),
                    model: WhisperModel::Base,
                },
            )
            .unwrap();
        assert_eq!(out.text, "hello from whisper");
        assert_eq!(out.metadata["model"], "base");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml whisper:: -- --nocapture`

Expected: FAIL until `pub mod whisper;` exists.

- [ ] **Step 3: Declare the module**

`mod.rs`: `pub mod whisper;`

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/domain/whisper.rs src-tauri/src/domain/mod.rs
git commit -m "feat: add WhisperEngine trait and error copy"
```

---

### Task 5: Prepare Whisper jobs (enum + ffmpeg check)

**Files:**
- Modify: `src-tauri/src/domain/jobs.rs` (PreparedTranscription, prepare, execute, process_* signatures, all tests)

- [ ] **Step 1: Write the failing tests**

Add these tests at the bottom of `jobs.rs` `mod tests`. They will not compile until signatures change — that is the red signal.

Also add test helpers in `mod tests` (near `setup`):

```rust
use crate::domain::ffmpeg::Ffmpeg;
use crate::domain::whisper::{ScriptedWhisper, WhisperEngine, ERR_FFMPEG_MISSING};
use crate::domain::whisper_models::{ModelDownloader, WhisperModel};

struct AvailableFfmpeg;
impl Ffmpeg for AvailableFfmpeg {
    fn is_available(&self) -> bool {
        true
    }
    fn to_wav_16k_mono(&self, src: &Path, dest: &Path) -> Result<(), String> {
        std::fs::copy(src, dest).map(|_| ()).map_err(|e| e.to_string())
    }
}

struct MissingFfmpeg;
impl Ffmpeg for MissingFfmpeg {
    fn is_available(&self) -> bool {
        false
    }
    fn to_wav_16k_mono(&self, _src: &Path, _dest: &Path) -> Result<(), String> {
        Err("should not convert".into())
    }
}

struct NoopDownloader;
impl ModelDownloader for NoopDownloader {
    fn download(&self, _url: &str, _dest: &Path) -> Result<(), String> {
        Err("download should not run".into())
    }
}

fn models_dir(dir: &tempfile::TempDir) -> std::path::PathBuf {
    let p = dir.path().join("whisper-models");
    std::fs::create_dir_all(&p).unwrap();
    p
}
```

New tests:

```rust
#[test]
fn whisper_prepare_succeeds_without_api_key() {
    let (dir, conn, storage, audio) = setup();
    settings::update_settings(
        &conn,
        crate::domain::models::UpdateSettingsInput {
            workspace_name: "Local".into(),
            default_provider: "whisper".into(),
            assemblyai_api_key: Some("".into()),
            language: "pt".into(),
            whisper_model: "tiny".into(),
        },
    )
    .unwrap();
    let job_id = create_uploads_and_jobs(
        &conn,
        &storage,
        &[NewUploadFile {
            filename: "clip.wav".into(),
            mime_type: "audio/wav".into(),
            size_bytes: 16,
            source_path: audio.path().to_string_lossy().into(),
        }],
    )
    .unwrap()[0]
        .id;
    let prepared = prepare_transcription(
        &conn,
        job_id,
        &AvailableFfmpeg,
        &models_dir(&dir),
    )
    .unwrap()
    .unwrap();
    match prepared {
        PreparedTranscription::Whisper {
            model,
            language,
            original_filename,
            ..
        } => {
            assert_eq!(model, WhisperModel::Tiny);
            assert_eq!(language.as_deref(), Some("pt"));
            assert_eq!(original_filename, "clip.wav");
        }
        PreparedTranscription::AssemblyAi { .. } => panic!("expected whisper"),
    }
    assert_eq!(
        get_job_detail(&conn, job_id).unwrap().unwrap().status,
        "processing"
    );
}

#[test]
fn whisper_prepare_fails_without_ffmpeg() {
    let (dir, conn, storage, audio) = setup();
    settings::update_settings(
        &conn,
        crate::domain::models::UpdateSettingsInput {
            workspace_name: "Local".into(),
            default_provider: "whisper".into(),
            assemblyai_api_key: None,
            language: "auto".into(),
            whisper_model: "base".into(),
        },
    )
    .unwrap();
    let job_id = create_uploads_and_jobs(
        &conn,
        &storage,
        &[NewUploadFile {
            filename: "clip.wav".into(),
            mime_type: "audio/wav".into(),
            size_bytes: 16,
            source_path: audio.path().to_string_lossy().into(),
        }],
    )
    .unwrap()[0]
        .id;
    let err = prepare_transcription(&conn, job_id, &MissingFfmpeg, &models_dir(&dir)).unwrap_err();
    assert_eq!(err, ERR_FFMPEG_MISSING);
    let detail = get_job_detail(&conn, job_id).unwrap().unwrap();
    assert_eq!(detail.status, "failed");
    assert_eq!(detail.error_message.as_deref(), Some(ERR_FFMPEG_MISSING));
}

#[test]
fn create_job_stamps_whisper_provider() {
    let (_dir, conn, storage, audio) = setup();
    settings::update_settings(
        &conn,
        crate::domain::models::UpdateSettingsInput {
            workspace_name: "Local".into(),
            default_provider: "whisper".into(),
            assemblyai_api_key: None,
            language: "auto".into(),
            whisper_model: "base".into(),
        },
    )
    .unwrap();
    let created = create_uploads_and_jobs(
        &conn,
        &storage,
        &[NewUploadFile {
            filename: "a.wav".into(),
            mime_type: "audio/wav".into(),
            size_bytes: 8,
            source_path: audio.path().to_string_lossy().into(),
        }],
    )
    .unwrap();
    let detail = get_job_detail(&conn, created[0].id).unwrap().unwrap();
    assert_eq!(detail.provider_key, "whisper");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml jobs:: -- --nocapture`

Expected: compile FAIL (`PreparedTranscription` has no `Whisper` variant; `prepare_transcription` takes 2 args).

- [ ] **Step 3: Implement prepare enum + update all call sites**

Replace `PreparedTranscription` and the prepare/execute/process functions with:

```rust
use crate::domain::ffmpeg::Ffmpeg;
use crate::domain::whisper::{self, WhisperEngine, WhisperTranscribeOptions};
use crate::domain::whisper_models::{self, ModelDownloader, WhisperModel};

#[derive(Debug, Clone)]
pub enum PreparedTranscription {
    AssemblyAi {
        audio_path: String,
        opts: TranscribeOptions,
    },
    Whisper {
        job_id: i64,
        audio_path: String,
        original_filename: String,
        model: WhisperModel,
        language: Option<String>,
        models_dir: PathBuf,
    },
}

pub struct TranscriptionDeps<'a> {
    pub http: &'a dyn HttpTransport,
    pub whisper: &'a dyn WhisperEngine,
    pub ffmpeg: &'a dyn Ffmpeg,
    pub models: &'a dyn ModelDownloader,
}

pub fn prepare_transcription(
    conn: &Connection,
    job_id: i64,
    ffmpeg: &dyn Ffmpeg,
    models_dir: &Path,
) -> Result<Option<PreparedTranscription>, String> {
    let detail = get_job_detail(conn, job_id)?
        .ok_or_else(|| format!("Job {job_id} not found"))?;

    if detail.status == "completed" {
        return Ok(None);
    }

    mark_job_processing(conn, job_id)?;

    let (audio_path, provider_key) = job_audio_path(conn, job_id)?;
    let settings = settings::get_settings(conn)?;
    let language = if settings.language == "auto" {
        None
    } else {
        Some(settings.language)
    };

    match provider_key.as_str() {
        "assemblyai" => {
            let api_key = match settings::assemblyai_api_key(conn)? {
                Some(k) => k,
                None => {
                    let msg = "AssemblyAI requires an API key in Settings".to_string();
                    mark_job_failed(conn, job_id, &msg)?;
                    return Err(msg);
                }
            };
            Ok(Some(PreparedTranscription::AssemblyAi {
                audio_path,
                opts: TranscribeOptions {
                    api_key,
                    language,
                    poll_interval: std::time::Duration::from_secs(3),
                    max_polls: 60,
                },
            }))
        }
        "whisper" => {
            if !ffmpeg.is_available() {
                let msg = whisper::ERR_FFMPEG_MISSING.to_string();
                mark_job_failed(conn, job_id, &msg)?;
                return Err(msg);
            }
            let model = WhisperModel::parse(&settings.whisper_model)?;
            Ok(Some(PreparedTranscription::Whisper {
                job_id,
                audio_path,
                original_filename: detail.original_filename,
                model,
                language,
                models_dir: models_dir.to_path_buf(),
            }))
        }
        other => {
            let msg = format!("Provider {other} is not available in this version");
            mark_job_failed(conn, job_id, &msg)?;
            Err(msg)
        }
    }
}

pub fn execute_transcription(
    deps: &TranscriptionDeps,
    prepared: &PreparedTranscription,
) -> Result<TranscriptionOutcome, String> {
    match prepared {
        PreparedTranscription::AssemblyAi { audio_path, opts } => {
            assemblyai::transcribe(deps.http, Path::new(audio_path), opts)
        }
        PreparedTranscription::Whisper { .. } => execute_whisper(deps, prepared),
    }
}

fn execute_whisper(
    _deps: &TranscriptionDeps,
    _prepared: &PreparedTranscription,
) -> Result<TranscriptionOutcome, String> {
    Err("whisper execute not implemented".into())
}

pub fn process_transcription_job_with_lock(
    db: &parking_lot::Mutex<Connection>,
    storage: &StorageRoot,
    deps: &TranscriptionDeps,
    models_dir: &Path,
    job_id: i64,
) -> Result<(), String> {
    let prepared = {
        let conn = db.lock();
        prepare_transcription(&conn, job_id, deps.ffmpeg, models_dir)?
    };
    let Some(prepared) = prepared else {
        return Ok(());
    };
    let network_result = execute_transcription(deps, &prepared);
    {
        let conn = db.lock();
        finish_transcription(&conn, storage, job_id, network_result)
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn process_transcription_job(
    conn: &Connection,
    storage: &StorageRoot,
    deps: &TranscriptionDeps,
    models_dir: &Path,
    job_id: i64,
) -> Result<(), String> {
    let prepared = prepare_transcription(conn, job_id, deps.ffmpeg, models_dir)?;
    let Some(prepared) = prepared else {
        return Ok(());
    };
    let network_result = execute_transcription(deps, &prepared);
    finish_transcription(conn, storage, job_id, network_result)
}
```

Need `use std::path::{Path, PathBuf};`.

Update **every** existing test call:

- `prepare_transcription(&conn, job_id)` → `prepare_transcription(&conn, job_id, &AvailableFfmpeg, &models_dir(&_dir))` using the `dir` from `setup()` (rename `_dir` to `dir` where needed).
- Poll override test:

```rust
let (dir, conn, storage, audio) = setup();
// ...
let mut prepared = prepare_transcription(&conn, job_id, &AvailableFfmpeg, &models_dir(&dir))
    .unwrap()
    .unwrap();
if let PreparedTranscription::AssemblyAi { opts, .. } = &mut prepared {
    opts.poll_interval = std::time::Duration::ZERO;
} else {
    panic!("assemblyai");
}
let whisper = ScriptedWhisper {
    text: String::new(),
    fail: true,
};
let ffmpeg = AvailableFfmpeg;
let models = NoopDownloader;
let deps = TranscriptionDeps {
    http: &ok,
    whisper: &whisper,
    ffmpeg: &ffmpeg,
    models: &models,
};
let outcome = execute_transcription(&deps, &prepared).unwrap();
```

- `process_transcription_job(&conn, &storage, &fail_t, job2)` → build `TranscriptionDeps` with `http: &fail_t` and dummy whisper/ffmpeg/downloader, pass `models_dir`.
- `process_transcription_job_with_lock(..., &transport, job_id)` → same deps pattern; `models_dir` from `dir.path().join("whisper-models")`.

AssemblyAI jobs must not call whisper/download. `NoopDownloader` / failing `ScriptedWhisper` is fine as long as they are not invoked.

`lib.rs` will not compile until Task 7. **Temporarily** keep it compiling: in `spawn_process_job`, this task may break `lib.rs`. Fix in this step with a thin adapter so `cargo test` (lib tests) still builds the lib:

In `lib.rs` `spawn_process_job`, you cannot construct `WhisperRsEngine` yet. Add a stub engine in `whisper.rs` (not cfg(test)):

```rust
pub struct UnimplementedWhisper;

impl WhisperEngine for UnimplementedWhisper {
    fn transcribe(&self, _: &Path, _: &WhisperTranscribeOptions) -> Result<TranscriptionOutcome, String> {
        Err(err_infer(WhisperModel::Base))
    }
}
```

And a downloader stub in `whisper_models.rs`:

```rust
pub struct UnimplementedDownloader;

impl ModelDownloader for UnimplementedDownloader {
    fn download(&self, _: &str, _: &Path) -> Result<(), String> {
        Err("model downloader not implemented".into())
    }
}
```

Then `lib.rs`:

```rust
use domain::ffmpeg::SystemFfmpeg;
use domain::jobs::TranscriptionDeps;
use domain::whisper::UnimplementedWhisper;
use domain::whisper_models::UnimplementedDownloader;

fn spawn_process_job(app: AppHandle, state: AppState, job_id: i64) {
    std::thread::spawn(move || {
        let transport = ReqwestTransport::default();
        let whisper = UnimplementedWhisper;
        let ffmpeg = SystemFfmpeg;
        let models = UnimplementedDownloader;
        let deps = TranscriptionDeps {
            http: &transport,
            whisper: &whisper,
            ffmpeg: &ffmpeg,
            models: &models,
        };
        let storage = state.storage();
        let result = jobs::process_transcription_job_with_lock(
            &state.db,
            &storage,
            &deps,
            &state.models_dir,
            job_id,
        );
        // existing emit...
    });
}
```

Add `pub models_dir: PathBuf` on `AppState` in this task. In `initialize`:

```rust
let models_dir = app_data_dir.join("whisper-models");
std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;
```

Pass it through `spawn_process_job` as `state.models_dir`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS, including the three new prepare tests. `execute_whisper` still returns “not implemented”; no execute-whisper test yet.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/domain/jobs.rs src-tauri/src/domain/whisper.rs src-tauri/src/domain/whisper_models.rs src-tauri/src/app_state.rs src-tauri/src/lib.rs
git commit -m "feat: prepare Whisper jobs without AssemblyAI key"
```

---

### Task 6: Execute Whisper (download, convert, infer, cleanup)

**Files:**
- Modify: `src-tauri/src/domain/jobs.rs` — replace `execute_whisper` stub + tests

- [ ] **Step 1: Write the failing tests**

Unit tests cannot ship 75MB ggml files, so `ensure_model` (not `ensure_artifact`) trusts a pre-written stub **only under `cfg(test)`**. SHA1 stays enforced in `ensure_artifact` (Task 3) and in production `ensure_model`.

In `whisper_models.rs` update `ensure_model`:

```rust
pub fn ensure_model(
    models_dir: &Path,
    model: WhisperModel,
    downloader: &dyn ModelDownloader,
) -> Result<PathBuf, String> {
    #[cfg(test)]
    {
        // cargo test only: execute tests drop a stub ggml file instead of 75MB weights.
        let target = models_dir.join(model.filename());
        if target.exists() {
            return Ok(target);
        }
    }
    ensure_artifact(
        models_dir,
        model.filename(),
        &model.url(),
        model.sha1_hex(),
        downloader,
    )
}
```

Add to `jobs.rs` tests:

```rust
struct FailingConvert;
impl Ffmpeg for FailingConvert {
    fn is_available(&self) -> bool {
        true
    }
    fn to_wav_16k_mono(&self, _src: &Path, _dest: &Path) -> Result<(), String> {
        Err("boom".into())
    }
}
```

Then add these tests:

```rust
#[test]
fn whisper_execute_writes_transcript_markdown() {
    let (dir, conn, storage, audio) = setup();
    settings::update_settings(
        &conn,
        crate::domain::models::UpdateSettingsInput {
            workspace_name: "Local".into(),
            default_provider: "whisper".into(),
            assemblyai_api_key: None,
            language: "auto".into(),
            whisper_model: "tiny".into(),
        },
    )
    .unwrap();
    let job_id = create_uploads_and_jobs(
        &conn,
        &storage,
        &[NewUploadFile {
            filename: "talk.wav".into(),
            mime_type: "audio/wav".into(),
            size_bytes: 16,
            source_path: audio.path().to_string_lossy().into(),
        }],
    )
    .unwrap()[0]
        .id;
    let md = models_dir(&dir);
    std::fs::write(md.join("ggml-tiny.bin"), b"stub-model").unwrap();

    let prepared = prepare_transcription(&conn, job_id, &AvailableFfmpeg, &md)
        .unwrap()
        .unwrap();
    let engine = ScriptedWhisper {
        text: "local whisper transcript".into(),
        fail: false,
    };
    let ffmpeg = AvailableFfmpeg;
    let models = NoopDownloader;
    let http = SeqTransport {
        step: AtomicUsize::new(0),
        fail: true,
    };
    let deps = TranscriptionDeps {
        http: &http,
        whisper: &engine,
        ffmpeg: &ffmpeg,
        models: &models,
    };
    let outcome = execute_transcription(&deps, &prepared).unwrap();
    finish_transcription(&conn, &storage, job_id, Ok(outcome)).unwrap();
    let detail = get_job_detail(&conn, job_id).unwrap().unwrap();
    assert_eq!(detail.status, "completed");
    assert_eq!(
        detail.transcript_text.as_deref(),
        Some("local whisper transcript")
    );
    let md_text = std::fs::read_to_string(detail.markdown_path.as_ref().unwrap()).unwrap();
    assert!(md_text.contains("Provider: whisper"));
    assert!(md_text.contains("local whisper transcript"));
}

#[test]
fn whisper_execute_convert_failure_does_not_call_engine() {
    let (dir, conn, storage, audio) = setup();
    settings::update_settings(
        &conn,
        crate::domain::models::UpdateSettingsInput {
            workspace_name: "Local".into(),
            default_provider: "whisper".into(),
            assemblyai_api_key: None,
            language: "auto".into(),
            whisper_model: "base".into(),
        },
    )
    .unwrap();
    let job_id = create_uploads_and_jobs(
        &conn,
        &storage,
        &[NewUploadFile {
            filename: "bad.ogg".into(),
            mime_type: "audio/ogg".into(),
            size_bytes: 16,
            source_path: audio.path().to_string_lossy().into(),
        }],
    )
    .unwrap()[0]
        .id;
    let md = models_dir(&dir);
    std::fs::write(md.join("ggml-base.bin"), b"stub-model").unwrap();
    let prepared = prepare_transcription(&conn, job_id, &AvailableFfmpeg, &md)
        .unwrap()
        .unwrap();
    let engine = ScriptedWhisper {
        text: "should not run".into(),
        fail: false,
    };
    let ffmpeg = FailingConvert;
    let models = NoopDownloader;
    let http = SeqTransport {
        step: AtomicUsize::new(0),
        fail: true,
    };
    let deps = TranscriptionDeps {
        http: &http,
        whisper: &engine,
        ffmpeg: &ffmpeg,
        models: &models,
    };
    let err = execute_transcription(&deps, &prepared).unwrap_err();
    assert_eq!(err, crate::domain::whisper::err_convert("bad.ogg"));
}

#[test]
fn whisper_execute_engine_failure_uses_infer_copy() {
    let (dir, conn, storage, audio) = setup();
    settings::update_settings(
        &conn,
        crate::domain::models::UpdateSettingsInput {
            workspace_name: "Local".into(),
            default_provider: "whisper".into(),
            assemblyai_api_key: None,
            language: "auto".into(),
            whisper_model: "small".into(),
        },
    )
    .unwrap();
    let job_id = create_uploads_and_jobs(
        &conn,
        &storage,
        &[NewUploadFile {
            filename: "x.wav".into(),
            mime_type: "audio/wav".into(),
            size_bytes: 16,
            source_path: audio.path().to_string_lossy().into(),
        }],
    )
    .unwrap()[0]
        .id;
    let md = models_dir(&dir);
    std::fs::write(md.join("ggml-small.bin"), b"stub").unwrap();
    let prepared = prepare_transcription(&conn, job_id, &AvailableFfmpeg, &md)
        .unwrap()
        .unwrap();
    let engine = ScriptedWhisper {
        text: String::new(),
        fail: true,
    };
    let ffmpeg = AvailableFfmpeg;
    let models = NoopDownloader;
    let http = SeqTransport {
        step: AtomicUsize::new(0),
        fail: true,
    };
    let deps = TranscriptionDeps {
        http: &http,
        whisper: &engine,
        ffmpeg: &ffmpeg,
        models: &models,
    };
    let err = execute_transcription(&deps, &prepared).unwrap_err();
    assert_eq!(err, crate::domain::whisper::err_infer(WhisperModel::Small));
}

#[test]
fn whisper_download_failure_uses_download_copy() {
    let (dir, conn, storage, audio) = setup();
    settings::update_settings(
        &conn,
        crate::domain::models::UpdateSettingsInput {
            workspace_name: "Local".into(),
            default_provider: "whisper".into(),
            assemblyai_api_key: None,
            language: "auto".into(),
            whisper_model: "base".into(),
        },
    )
    .unwrap();
    let job_id = create_uploads_and_jobs(
        &conn,
        &storage,
        &[NewUploadFile {
            filename: "x.wav".into(),
            mime_type: "audio/wav".into(),
            size_bytes: 16,
            source_path: audio.path().to_string_lossy().into(),
        }],
    )
    .unwrap()[0]
        .id;
    let md = models_dir(&dir);
    // no stub file → ensure_model hits downloader
    let prepared = prepare_transcription(&conn, job_id, &AvailableFfmpeg, &md)
        .unwrap()
        .unwrap();
    let engine = ScriptedWhisper {
        text: String::new(),
        fail: true,
    };
    let ffmpeg = AvailableFfmpeg;
    let models = NoopDownloader;
    let http = SeqTransport {
        step: AtomicUsize::new(0),
        fail: true,
    };
    let deps = TranscriptionDeps {
        http: &http,
        whisper: &engine,
        ffmpeg: &ffmpeg,
        models: &models,
    };
    let err = execute_transcription(&deps, &prepared).unwrap_err();
    assert_eq!(err, crate::domain::whisper::err_download(WhisperModel::Base));
}
```

Add a lock regression for Whisper convert (DB free during ffmpeg):

```rust
#[test]
fn process_with_lock_releases_db_during_whisper_convert() {
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, Instant};

    struct BlockingFfmpeg {
        entered: Arc<std::sync::Barrier>,
        list_done: Arc<std::sync::Barrier>,
    }
    impl Ffmpeg for BlockingFfmpeg {
        fn is_available(&self) -> bool {
            true
        }
        fn to_wav_16k_mono(&self, src: &Path, dest: &Path) -> Result<(), String> {
            self.entered.wait();
            self.list_done.wait();
            std::fs::copy(src, dest).map(|_| ()).map_err(|e| e.to_string())
        }
    }

    let dir = tempdir().unwrap();
    let conn = db::open(&dir.path().join("lock-w.db")).unwrap();
    let storage = StorageRoot::new(dir.path().join("storage"));
    settings::update_settings(
        &conn,
        crate::domain::models::UpdateSettingsInput {
            workspace_name: "Local".into(),
            default_provider: "whisper".into(),
            assemblyai_api_key: None,
            language: "auto".into(),
            whisper_model: "tiny".into(),
        },
    )
    .unwrap();
    let audio = NamedTempFile::new().unwrap();
    std::fs::write(audio.path(), b"RIFF....WAVEfmt ").unwrap();
    let job_id = create_uploads_and_jobs(
        &conn,
        &storage,
        &[NewUploadFile {
            filename: "lock.wav".into(),
            mime_type: "audio/wav".into(),
            size_bytes: 16,
            source_path: audio.path().to_string_lossy().into(),
        }],
    )
    .unwrap()[0]
        .id;
    drop(conn);
    let md = dir.path().join("whisper-models");
    std::fs::create_dir_all(&md).unwrap();
    std::fs::write(md.join("ggml-tiny.bin"), b"stub").unwrap();

    let db = Arc::new(Mutex::new(db::open(&dir.path().join("lock-w.db")).unwrap()));
    let entered = Arc::new(std::sync::Barrier::new(2));
    let list_done = Arc::new(std::sync::Barrier::new(2));
    let ffmpeg = BlockingFfmpeg {
        entered: entered.clone(),
        list_done: list_done.clone(),
    };
    let engine = ScriptedWhisper {
        text: "unlocked whisper".into(),
        fail: false,
    };
    let models = NoopDownloader;
    let http = SeqTransport {
        step: AtomicUsize::new(0),
        fail: true,
    };

    let db_worker = db.clone();
    let storage_worker = StorageRoot::new(dir.path().join("storage"));
    let md_worker = md.clone();
    let worker = thread::spawn(move || {
        let deps = TranscriptionDeps {
            http: &http,
            whisper: &engine,
            ffmpeg: &ffmpeg,
            models: &models,
        };
        process_transcription_job_with_lock(&db_worker, &storage_worker, &deps, &md_worker, job_id)
    });

    entered.wait();
    let start = Instant::now();
    {
        let conn = db.lock();
        let list = list_jobs(&conn).unwrap();
        assert_eq!(list[0].status, "processing");
    }
    let elapsed = start.elapsed();
    list_done.wait();
    assert!(
        elapsed < Duration::from_millis(500),
        "list_jobs blocked too long ({elapsed:?})"
    );
    worker.join().unwrap().unwrap();
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml jobs::whisper_execute -- --nocapture`

Expected: FAIL (`whisper execute not implemented`).

- [ ] **Step 3: Implement `execute_whisper`**

```rust
fn execute_whisper(
    deps: &TranscriptionDeps,
    prepared: &PreparedTranscription,
) -> Result<TranscriptionOutcome, String> {
    let PreparedTranscription::Whisper {
        job_id,
        audio_path,
        original_filename,
        model,
        language,
        models_dir,
    } = prepared
    else {
        return Err("not a whisper job".into());
    };

    let model_path =
        whisper_models::ensure_model(models_dir, *model, deps.models).map_err(|_| whisper::err_download(*model))?;

    let tmp = std::env::temp_dir().join(format!(
        "mass-transcriptor-whisper-{}-{}.wav",
        job_id,
        std::process::id()
    ));
    struct DeleteOnDrop(std::path::PathBuf);
    impl Drop for DeleteOnDrop {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _guard = DeleteOnDrop(tmp.clone());

    deps.ffmpeg
        .to_wav_16k_mono(Path::new(audio_path), &tmp)
        .map_err(|_| whisper::err_convert(original_filename))?;

    let opts = WhisperTranscribeOptions {
        model_path,
        language: language.clone(),
        model: *model,
    };
    deps.whisper
        .transcribe(&tmp, &opts)
        .map_err(|_| whisper::err_infer(*model))
}
```

Call order is download (`ensure_model`) → convert → infer, matching the spec. Convert-failure tests pre-write a stub model so `ensure_model` returns immediately under `cfg(test)` and never calls the engine. Download-failure tests omit the stub so `NoopDownloader` errors before convert.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/domain/jobs.rs src-tauri/src/domain/whisper_models.rs
git commit -m "feat: execute local Whisper jobs via injected engine"
```

---

### Task 7: Production whisper-rs + HTTP downloader

**Files:**
- Modify: `src-tauri/Cargo.toml` — add `whisper-rs`, `hound`
- Modify: `src-tauri/src/domain/whisper.rs` — `WhisperRsEngine`
- Modify: `src-tauri/src/domain/whisper_models.rs` — `ReqwestModelDownloader`
- Modify: `src-tauri/src/lib.rs` — use production types; delete stubs from the worker
- Modify: `src-tauri/src/domain/whisper.rs` / `whisper_models.rs` — remove `UnimplementedWhisper` / `UnimplementedDownloader` if unused

Requires **cmake** and a C compiler on the machine (`brew install cmake` on macOS). First compile will be slow.

- [ ] **Step 1: Add dependencies**

`Cargo.toml`:

```toml
sha1 = "0.10"
whisper-rs = { version = "0.16.0", default-features = false }
hound = "3.5"
```

Do not add a unit test that loads a real ggml model.

- [ ] **Step 2: Run `cargo test --manifest-path src-tauri/Cargo.toml`**

Expected: PASS (deps compile; no behavior change yet). If cmake is missing, install it and re-run. Do not proceed without a green compile.

- [ ] **Step 3: Production implementations**

`ReqwestModelDownloader` in `whisper_models.rs`:

```rust
pub struct ReqwestModelDownloader {
    client: reqwest::blocking::Client,
}

impl Default for ReqwestModelDownloader {
    fn default() -> Self {
        Self {
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(600))
                .user_agent("mass-transcriptor-tauri")
                .build()
                .expect("reqwest client"),
        }
    }
}

impl ModelDownloader for ReqwestModelDownloader {
    fn download(&self, url: &str, dest: &Path) -> Result<(), String> {
        let mut resp = self
            .client
            .get(url)
            .send()
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }
        let mut file = std::fs::File::create(dest).map_err(|e| e.to_string())?;
        std::io::copy(&mut resp, &mut file).map_err(|e| e.to_string())?;
        Ok(())
    }
}
```

`WhisperRsEngine` in `whisper.rs` (not behind cfg(test)):

```rust
pub struct WhisperRsEngine;

impl WhisperEngine for WhisperRsEngine {
    fn transcribe(
        &self,
        wav_path: &Path,
        opts: &WhisperTranscribeOptions,
    ) -> Result<TranscriptionOutcome, String> {
        let model_path = opts
            .model_path
            .to_str()
            .ok_or_else(|| "invalid model path".to_string())?;
        let ctx = whisper_rs::WhisperContext::new_with_params(
            model_path,
            whisper_rs::WhisperContextParameters::default(),
        )
        .map_err(|e| e.to_string())?;
        let mut state = ctx.create_state().map_err(|e| e.to_string())?;

        let mut reader = hound::WavReader::open(wav_path).map_err(|e| e.to_string())?;
        let samples_i16: Vec<i16> = reader
            .samples::<i16>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let mut audio = vec![0.0f32; samples_i16.len()];
        whisper_rs::convert_integer_to_float_audio(&samples_i16, &mut audio)
            .map_err(|e| e.to_string())?;

        let mut params =
            whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
        params.set_translate(false);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        match opts.language.as_deref() {
            Some(lang) => params.set_language(Some(lang)),
            None => params.set_language(None),
        }

        state
            .full(params, &audio)
            .map_err(|e| e.to_string())?;
        let n = state.full_n_segments().map_err(|e| e.to_string())?;
        let mut text = String::new();
        for i in 0..n {
            text.push_str(&state.full_get_segment_text(i).map_err(|e| e.to_string())?);
        }
        Ok(TranscriptionOutcome {
            text: text.trim().to_string(),
            metadata: serde_json::json!({
                "model": opts.model.as_str(),
                "language": opts.language,
            }),
        })
    }
}
```

If `convert_integer_to_float_audio` signature differs, convert manually: `audio[i] = samples_i16[i] as f32 / 32768.0`.

If `set_language(None)` does not compile, omit the call for auto-detect.

`lib.rs` worker:

```rust
let transport = ReqwestTransport::default();
let whisper = domain::whisper::WhisperRsEngine;
let ffmpeg = domain::ffmpeg::SystemFfmpeg;
let models = domain::whisper_models::ReqwestModelDownloader::default();
let deps = jobs::TranscriptionDeps {
    http: &transport,
    whisper: &whisper,
    ffmpeg: &ffmpeg,
    models: &models,
};
let storage = state.storage();
let result = jobs::process_transcription_job_with_lock(
    &state.db,
    &storage,
    &deps,
    &state.models_dir,
    job_id,
);
```

Delete `UnimplementedWhisper` and `UnimplementedDownloader`.

- [ ] **Step 4: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS. Tests still never construct `WhisperRsEngine`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/domain/whisper.rs src-tauri/src/domain/whisper_models.rs src-tauri/src/lib.rs
git commit -m "feat: wire whisper-rs, ffmpeg, and ggml downloads"
```

---

### Task 8: Settings and Upload UI

**Files:**
- Modify: `src/lib/api.ts`
- Modify: `src/pages/SettingsPage.tsx`
- Modify: `src/pages/UploadPage.tsx`

- [ ] **Step 1: Types**

`api.ts`:

```ts
export type AppSettings = {
  workspaceName: string;
  defaultProvider: string;
  assemblyaiApiKey: string | null;
  hasApiKey: boolean;
  language: string;
  whisperModel: string;
};

export type UpdateSettingsInput = {
  workspaceName: string;
  defaultProvider: string;
  assemblyaiApiKey?: string | null;
  language: string;
  whisperModel: string;
};
```

- [ ] **Step 2: Settings page**

- `useState` for `whisperModel` default `"base"`.
- Load `s.whisperModel` in `useEffect`.
- Send `whisperModel` in save payload.
- Provider `<option value="whisper">whisper</option>` — **not disabled**, no “(not available)”.
- When `defaultProvider === "whisper"`, show:

```tsx
<label className="settings-form__field">
  <span>Whisper model</span>
  <select
    id="settings-whisper-model"
    value={whisperModel}
    onChange={(e) => setWhisperModel(e.target.value)}
    aria-label="Whisper model"
  >
    <option value="tiny">tiny (~75 MB)</option>
    <option value="base">base (~142 MB)</option>
    <option value="small">small (~466 MB)</option>
  </select>
</label>
<p className="settings-shell__lede">
  The first Whisper job for a model size downloads a ggml file into app data.
  ffmpeg must be installed and available as the <code>ffmpeg</code> command.
</p>
```

- Intro note: keep AssemblyAI key copy; add that Whisper is local and does not use that key.

- [ ] **Step 3: Upload copy**

Replace the subtitle with:

```tsx
Drop audio or short video files here. Files are transcribed with the default
provider from Settings.
```

- [ ] **Step 4: Typecheck**

Run: `npx vitest run` and `npx tsc --noEmit`

Expected: PASS / no errors.

- [ ] **Step 5: Commit**

```bash
git add src/lib/api.ts src/pages/SettingsPage.tsx src/pages/UploadPage.tsx
git commit -m "feat: enable Whisper provider and model in Settings"
```

---

### Task 9: README

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update stack + features**

Stack bullet: add local **Whisper (whisper.cpp)** via Settings; ffmpeg required on PATH.

Features: Settings include Whisper model `tiny|base|small`; first job downloads ggml weights into app data (`whisper-models/`).

New section after AssemblyAI mention:

```markdown
### Whisper (local)

1. Install `ffmpeg` and confirm `ffmpeg -version` works in a terminal.
2. Open Settings, set default provider to `whisper`, pick `tiny`, `base`, or `small` (default `base`).
3. No API key is required for Whisper.
4. The first job that uses a given model size downloads `ggml-{size}.bin` from Hugging Face into the app data directory (tiny ~75 MB, base ~142 MB, small ~466 MB). Later jobs reuse the file.
5. If ffmpeg is missing, the job fails with an install/retry message.

Building the desktop app compiles whisper.cpp (cmake + a C/C++ compiler required).
```

Security: ggml files and API keys stay in the OS app data directory; do not commit them.

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: document local Whisper provider and ffmpeg"
```

---

## Self-review vs spec

| Spec item | Task |
| --- | --- |
| Enable Settings Whisper option | 8 |
| Persist `whisper_model` tiny/base/small default base | 1 |
| Job pipeline Whisper path | 5–6 |
| Download ggml into app data + SHA1 | 3, 6, 7 |
| ffmpeg 16 kHz mono WAV, original untouched | 2, 6 |
| Markdown `Provider: whisper` | 6 |
| Error copy ffmpeg / download / convert / infer | 4, 5, 6 |
| Domain tests with fakes | 1–6 |
| No Metal / no medium+ / no progress UI | not implemented |
| Prepare without AssemblyAI key | 5 |
| Mutex not held during download/convert/infer | 6 lock test |
| `AppState.models_dir` | 5 |
| `TranscriptionDeps` | 5 |
| Production whisper-rs + reqwest download | 7 |
| Upload copy | 8 |
| README | 9 |
| Retry uses current Settings model/language | 5 (prepare reads settings; retry unchanged) |

SHA1 pins live in `WhisperModel::sha1_hex`. If a real Hugging Face file fails verification on first manual run, hash the downloaded file and update the three constants + spec table in a follow-up — do not skip verification.

`#[cfg(test)]` early-return in `ensure_model` when the stub file exists is **only** so execute tests can avoid 75MB fixtures. `ensure_artifact` still enforces SHA1 (covered in Task 3). Production `ensure_model` never compiles that branch in the release binary (`cfg(test)` is off).
