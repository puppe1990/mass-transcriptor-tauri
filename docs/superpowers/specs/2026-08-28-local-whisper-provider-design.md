# Local Whisper Provider Design

**Date:** 2026-08-28  
**App:** Mass Transcriptor (Tauri + SQLite)

## Goal

Make `whisper` a working default transcription provider: local whisper.cpp in-process, model size chosen in Settings (`tiny` / `base` / `small`), model file downloaded on first job that needs it, ffmpeg on PATH to convert media to 16 kHz mono WAV.

## Scope

Included:

- Enable the Settings provider option `whisper` (today it is disabled)
- Persist `whisper_model` (`tiny` | `base` | `small`, default `base`)
- Job pipeline path for `provider_key = "whisper"`
- Download ggml weights into app data when missing
- Convert source media with ffmpeg, transcribe with whisper.cpp, write the same markdown result format as AssemblyAI
- Actionable job-failure messages for missing ffmpeg, failed download, failed conversion, failed inference
- Domain tests with injected fakes (no real network, no real ffmpeg, no real model)

Excluded:

- Metal / CUDA / Core ML acceleration
- Models larger than `small` (`medium`, `large-v3`, quantized variants)
- Bundling ffmpeg or ggml files in the app installer
- Download progress UI or a `downloading` job status
- Per-job model or language override
- OpenAI Whisper HTTP API
- Python `openai-whisper` sidecar
- Changing AssemblyAI behavior except that its API key is not required when the default provider is Whisper

## Product behavior

Settings:

- Default provider select includes `assemblyai` and `whisper` (Whisper is not disabled).
- When provider is `whisper`, a model select is shown: `tiny` (~75 MB), `base` (~142 MB), `small` (~466 MB). Default `base`.
- Short copy: the first job that uses a given size downloads that ggml file into app data; ffmpeg must be installed and on PATH.
- AssemblyAI API key remains optional to save when provider is Whisper. It stays required at run time only for jobs whose `provider_key` is `assemblyai`.
- Transcription language stays the existing workspace setting: `auto` | `pt` | `en` | `es`. `auto` means Whisper detects language; otherwise the code is passed through.

Upload:

- Subtitle no longer says every file is transcribed with AssemblyAI. It says files are transcribed with the default provider from Settings.

Jobs:

- New uploads stamp `provider_key` from `default_provider` at create time (unchanged).
- Whisper jobs do not need an AssemblyAI key.
- While downloading a model or running inference the job stays `processing`. No extra status.
- Failed Whisper jobs are retryable, same as today.
- Retry of an old job with `provider_key = whisper` uses the **current** Settings model and language (jobs do not snapshot model).

Results:

- Markdown is unchanged: `# Transcript`, source filename, `Provider: whisper`, body text.

## Architecture

Keep the existing three-phase worker. Do not add a second queue.

1. `prepare` (short SQLite lock): mark `processing`, load audio path, settings, and provider-specific inputs. Fail fast if ffmpeg is missing for Whisper. Do **not** download the model or convert audio here.
2. `execute` (no SQLite lock): download model if needed, ffmpeg convert, whisper.cpp infer.
3. `finish` (short SQLite lock): persist `TranscriptionOutcome` or mark `failed`.

`PreparedTranscription` becomes an enum:

```rust
pub enum PreparedTranscription {
    AssemblyAi {
        audio_path: String,
        opts: TranscribeOptions,
    },
    Whisper {
        audio_path: String,
        original_filename: String,
        model: WhisperModel,       // Tiny | Base | Small
        language: Option<String>,  // None = auto
        models_dir: PathBuf,
    },
}
```

`execute_transcription` takes a small deps struct instead of growing positional arguments:

```rust
pub struct TranscriptionDeps<'a> {
    pub http: &'a dyn HttpTransport,
    pub whisper: &'a dyn WhisperEngine,
    pub ffmpeg: &'a dyn Ffmpeg,
    pub models: &'a dyn ModelDownloader,
}
```

Production worker (`process_transcription_job_with_lock`) uses `ReqwestTransport`, a `whisper-rs` engine, a `std::process::Command` ffmpeg adapter, and an HTTP model downloader. Tests inject fakes.

Build: add crate `whisper-rs` (whisper.cpp via cmake). Developers need cmake and a C/C++ compiler. Inference is CPU-only in this version.

App data layout:

```
{app_data_dir}/
  mass-transcriptor.db
  storage/
  whisper-models/
    ggml-tiny.bin
    ggml-base.bin
    ggml-small.bin
```

`AppState` gains `pub models_dir: PathBuf`. `initialize` creates `whisper-models/` next to `storage/` and stores that path.

## Components

### `domain/whisper.rs`

- Trait `WhisperEngine`: `transcribe(wav_path, WhisperTranscribeOptions) -> Result<TranscriptionOutcome, String>`
- `WhisperTranscribeOptions { model_path, language: Option<String> }`
- Production impl wraps `whisper-rs`: load the ggml model from `model_path`, run transcription on the WAV, return `text` plus metadata `{ "language": ..., "model": "tiny"|"base"|"small" }`.
- Tests never construct the production impl.

### `domain/ffmpeg.rs`

- Trait `Ffmpeg`:
  - `is_available(&self) -> bool` — production: `ffmpeg -version` succeeds
  - `to_wav_16k_mono(&self, src: &Path, dest: &Path) -> Result<(), String>` — production command:

    `ffmpeg -y -i <src> -ac 1 -ar 16000 -c:a pcm_s16le <dest>`

- Temp WAV path: OS temp dir, unique name (`mass-transcriptor-whisper-{job_id}-{pid}.wav`). Always deleted after `execute`, success or failure.
- Original file under `storage/` is never modified.

### `domain/whisper_models.rs`

Allowed models and pinned artifacts (Hugging Face `ggerganov/whisper.cpp`, `resolve/main`):

| Settings value | File | Approx size | SHA1 |
| --- | --- | --- | --- |
| `tiny` | `ggml-tiny.bin` | 75 MiB | `bd577a113a864445d4c299885e0cb97d4ba92b5f` |
| `base` | `ggml-base.bin` | 142 MiB | `465707469ff3a37a2b9b8d8f89f2f99de7299dac` |
| `small` | `ggml-small.bin` | 466 MiB | `55356645c2b361a969dfd0ef2c5a50d530afd8d5` |

URL pattern: `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{file}`

`ensure_model(models_dir, model, downloader) -> PathBuf`:

1. Target path = `models_dir/ggml-{name}.bin`.
2. If the file exists and SHA1 matches, return it.
3. If it exists but SHA1 mismatches, delete it and download again.
4. Download to `ggml-{name}.bin.partial`, verify SHA1, then rename into place.
5. On any failure, delete the `.partial` (and do not leave a corrupt `.bin`).

Trait `ModelDownloader`: `download(url: &str, dest: &Path) -> Result<(), String>`.

### Settings and schema

`app_settings` gains `whisper_model TEXT NOT NULL DEFAULT 'base'`.

Migration: `ALTER TABLE app_settings ADD COLUMN whisper_model TEXT NOT NULL DEFAULT 'base';` guarded so existing local DBs upgrade.

Allowed providers remain `assemblyai` | `whisper`.  
Allowed `whisper_model` values: `tiny` | `base` | `small`.

`AppSettings` / `UpdateSettingsInput` / frontend `api.ts` include `whisperModel`.

Selecting Whisper does not require an API key. Clearing or omitting `assemblyaiApiKey` still follows today’s omit-keeps-existing / empty-clears rules.

### `domain/jobs.rs`

- Remove the hard fail `Provider {key} is not available in this version` for `whisper`.
- Unknown provider keys still fail with that message.

`prepare_transcription` signature:

```rust
pub fn prepare_transcription(
    conn: &Connection,
    job_id: i64,
    ffmpeg: &dyn Ffmpeg,
    models_dir: &Path,
) -> Result<Option<PreparedTranscription>, String>
```

- Whisper branch: `ffmpeg.is_available()` must be true; no AssemblyAI key; `language` is `None` when settings language is `auto`, else `Some(code)`; `model` from `whisper_model`; `models_dir` copied onto the enum variant.
- AssemblyAI branch: unchanged (still requires API key). Ignores ffmpeg except that the parameter exists.
- Unknown `provider_key`: fail with the existing “not available in this version” message.
- Retry does not change `provider_key`. A retried Whisper job reads **current** Settings for model and language at prepare time.

`execute_transcription(deps, prepared)` matches on `PreparedTranscription`.

`process_transcription_job` / `process_transcription_job_with_lock` take `TranscriptionDeps` and `models_dir`.

### UI

- `SettingsPage.tsx`: enable Whisper; show model `<select id="settings-whisper-model">` when provider is `whisper`; helper text about first-job download and ffmpeg.
- `UploadPage.tsx`: provider-agnostic subtitle.
- Job/batch pages: no new chrome; they already show `providerKey`.

### README

Document Whisper as a local option: install ffmpeg, pick Whisper + model in Settings, first job downloads ggml weights, no extra API key.

## Data flow

```
Settings save (whisper, base, language)
        │
Upload ─┼─ create job provider_key=whisper
        │
Worker prepare (DB lock)
        │  ffmpeg missing? → failed + message
        │
Worker execute (no DB lock)
        │  ensure_model (download if needed)
        │  ffmpeg → temp 16 kHz mono WAV
        │  whisper-rs transcribe
        │  delete temp WAV
        │
Worker finish (DB lock)
           success → transcription_results + markdown
           error   → failed, retryable
```

Download, ffmpeg, and inference must not hold the SQLite mutex (same rule as AssemblyAI HTTP).

## Error handling

Copy is English, matching the rest of the desktop UI. Job `error_message` stays truncated to 500 characters as today.

| Failure | Job status | Message |
| --- | --- | --- |
| `ffmpeg` not on PATH / `-version` fails | `failed`, retryable | Whisper needs ffmpeg installed on this machine. Install ffmpeg, confirm the `ffmpeg` command works in a terminal, and retry this job. |
| Model download or SHA1 mismatch | `failed`, retryable | Could not download Whisper model `{tiny\|base\|small}` (~75/142/466 MB) from Hugging Face. Check the network and retry. Incomplete files are not kept. |
| ffmpeg conversion fails | `failed`, retryable | ffmpeg could not convert `{filename}` to 16 kHz WAV. Check that the file plays in a media player and that ffmpeg is up to date, then retry. |
| whisper.cpp / engine error | `failed`, retryable | Whisper transcription (`{tiny\|base\|small}`) failed for this file. Retry the job; if it keeps failing, switch to `tiny` or use AssemblyAI. |
| AssemblyAI job with empty key | `failed`, retryable | AssemblyAI requires an API key in Settings. (unchanged) |
| Unknown provider | `failed`, retryable | Provider `{key}` is not available in this version. (unchanged, Whisper no longer hits this) |
| Invalid `whisper_model` on save | Settings error, no job | Whisper model '{value}' is not allowed |

No `downloading` status. No byte progress in the UI.

## Testing

All domain tests use fakes. CI does not download ggml files or call real ffmpeg.

Settings:

- Persist `default_provider=whisper` and `whisper_model=small`; reload matches.
- Reject `whisper_model=medium` (or any value outside `tiny|base|small`).
- Omit AssemblyAI key while switching to Whisper; existing key is kept.
- Default on fresh DB is `whisper_model=base`.

Prepare:

- Whisper prepare succeeds with no AssemblyAI key when ffmpeg fake reports available.
- Whisper prepare fails with the ffmpeg-missing message when ffmpeg fake reports unavailable; job is `failed`.
- AssemblyAI prepare still fails without an API key.

Models:

- Existing valid `.bin` (SHA1 matches) does not call the downloader.
- Missing file: fake downloader writes bytes; `ensure_model` returns the target path.
- Downloader error: no `.bin` and no `.partial` left on disk.
- Corrupt existing file (wrong SHA1): deleted and downloaded again.

Execute + finish:

- Fake Whisper engine returns text; finish writes markdown containing `Provider: whisper` and the transcript.
- Conversion failure uses the ffmpeg-conversion message and does not call the engine.
- AssemblyAI path still uses `HttpTransport` only; Whisper path does not call HTTP transcript endpoints.

Worker lock:

- Existing regression (mutex not held during I/O) still passes; Whisper download/convert/infer happen after the prepare lock is released.

Frontend:

- No new E2E. Settings markup: Whisper `<option>` is not `disabled`; model select `id="settings-whisper-model"` with `tiny`, `base`, `small`.

## Risks

- First `cargo build` / `tauri dev` is slower because whisper.cpp compiles via cmake.
- First Whisper job on a machine needs disk (up to ~466 MB for `small`) and network.
- ffmpeg must be installed by the user; the failure message is the recovery path.
- SHA1 pins can fail if Hugging Face replaces a file; the job fails retryably and the spec’s table is the source of truth until updated.

## Success criteria

- User can set default provider to Whisper, pick `tiny`/`base`/`small`, save without an AssemblyAI key.
- A Whisper job on a machine with ffmpeg completes and shows transcript markdown with `Provider: whisper`.
- A Whisper job without ffmpeg fails immediately with the ffmpeg install message and can be retried after install.
- AssemblyAI jobs behave as they do today.
- `npm test` (cargo + vitest) passes without network, ffmpeg, or ggml files.
