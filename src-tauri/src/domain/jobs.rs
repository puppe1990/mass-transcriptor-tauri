//! Upload → job/batch lifecycle against SQLite (no tenants/users).

use crate::domain::assemblyai::{self, HttpTransport, TranscribeOptions};
use crate::domain::markdown;
use crate::domain::models::{
    BatchDetail, CreatedJob, JobDetail, JobSummary, NewUploadFile, TranscriptionOutcome,
};
use crate::domain::settings;
use crate::domain::storage::StorageRoot;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

const AUDIO_EXTS: &[&str] = &[
    "wav", "mp3", "ogg", "opus", "m4a", "flac", "webm", "aac", "wma", "mpga", "oga",
];
const VIDEO_EXTS: &[&str] = &["mp4", "mov", "mkv"];

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn extension(filename: &str) -> String {
    Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
}

pub fn is_supported(filename: &str) -> bool {
    let ext = extension(filename);
    AUDIO_EXTS.contains(&ext.as_str()) || VIDEO_EXTS.contains(&ext.as_str())
}

#[allow(dead_code)]
pub fn is_video(filename: &str, mime_type: &str) -> bool {
    VIDEO_EXTS.contains(&extension(filename).as_str()) || mime_type.starts_with("video/")
}

fn is_retryable(status: &str) -> bool {
    status == "failed"
}

/// Create uploads + jobs (batch when multiple files). Returns created jobs.
pub fn create_uploads_and_jobs(
    conn: &Connection,
    storage: &StorageRoot,
    files: &[NewUploadFile],
) -> Result<Vec<CreatedJob>, String> {
    if files.is_empty() {
        return Err("No files provided".into());
    }
    for f in files {
        if !is_supported(&f.filename) {
            return Err(format!(
                "Unsupported file type: {}. Use common audio formats or MP4/MOV/MKV.",
                f.filename
            ));
        }
        if !Path::new(&f.source_path).exists() {
            return Err(format!("Source file missing: {}", f.source_path));
        }
    }

    let settings = settings::get_settings(conn)?;
    let provider = settings.default_provider;
    let ts = now();

    let batch_id: Option<i64> = if files.len() > 1 {
        conn.execute(
            "INSERT INTO job_batches (created_at) VALUES (?1)",
            params![ts],
        )
        .map_err(|e| e.to_string())?;
        Some(conn.last_insert_rowid())
    } else {
        None
    };

    let mut created = Vec::new();

    for file in files {
        conn.execute(
            "INSERT INTO uploads (original_filename, mime_type, size_bytes, audio_path, created_at)
             VALUES (?1, ?2, ?3, 'pending', ?4)",
            params![file.filename, file.mime_type, file.size_bytes, ts],
        )
        .map_err(|e| e.to_string())?;
        let upload_id = conn.last_insert_rowid();

        let audio_path = storage
            .write_audio_from_path(upload_id, &file.filename, Path::new(&file.source_path))?;
        let audio_path_str = audio_path.to_string_lossy().to_string();

        conn.execute(
            "UPDATE uploads SET audio_path = ?1 WHERE id = ?2",
            params![audio_path_str, upload_id],
        )
        .map_err(|e| e.to_string())?;

        conn.execute(
            "INSERT INTO transcription_jobs
                (upload_id, batch_id, provider_key, status, error_message, started_at, completed_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'queued', NULL, NULL, NULL, ?4, ?4)",
            params![upload_id, batch_id, provider, ts],
        )
        .map_err(|e| e.to_string())?;
        let job_id = conn.last_insert_rowid();

        created.push(CreatedJob {
            id: job_id,
            upload_id,
            batch_id,
            status: "queued".into(),
            original_filename: file.filename.clone(),
        });
    }

    Ok(created)
}

pub fn list_jobs(conn: &Connection) -> Result<Vec<JobSummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT j.id, j.status, j.provider_key, j.batch_id, j.upload_id,
                    u.original_filename, j.error_message, j.created_at
             FROM transcription_jobs j
             INNER JOIN uploads u ON u.id = j.upload_id
             ORDER BY j.id DESC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            let status: String = row.get(1)?;
            Ok(JobSummary {
                id: row.get(0)?,
                status: status.clone(),
                provider_key: row.get(2)?,
                batch_id: row.get(3)?,
                upload_id: row.get(4)?,
                original_filename: row.get(5)?,
                error_message: row.get(6)?,
                created_at: row.get(7)?,
                retryable: is_retryable(&status),
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

/// Flat jobs collapsed into batch + single UI rows.
pub fn list_job_rows(conn: &Connection) -> Result<Vec<crate::domain::grouping::JobListRow>, String> {
    let jobs = list_jobs(conn)?;
    Ok(crate::domain::grouping::build_job_list_rows(&jobs))
}

pub fn get_job_detail(conn: &Connection, job_id: i64) -> Result<Option<JobDetail>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT j.id, j.status, j.provider_key, j.batch_id, j.upload_id,
                    u.original_filename, j.error_message, j.created_at, j.started_at, j.completed_at,
                    r.markdown_path, r.transcript_text
             FROM transcription_jobs j
             INNER JOIN uploads u ON u.id = j.upload_id
             LEFT JOIN transcription_results r ON r.job_id = j.id
             WHERE j.id = ?1",
        )
        .map_err(|e| e.to_string())?;

    let detail = stmt
        .query_row(params![job_id], |row| {
            let status: String = row.get(1)?;
            Ok(JobDetail {
                id: row.get(0)?,
                status: status.clone(),
                provider_key: row.get(2)?,
                batch_id: row.get(3)?,
                upload_id: row.get(4)?,
                original_filename: row.get(5)?,
                error_message: row.get(6)?,
                created_at: row.get(7)?,
                started_at: row.get(8)?,
                completed_at: row.get(9)?,
                markdown_path: row.get(10)?,
                transcript_text: row.get(11)?,
                retryable: is_retryable(&status),
            })
        })
        .optional()
        .map_err(|e| e.to_string())?;

    Ok(detail)
}

pub fn get_batch_detail(conn: &Connection, batch_id: i64) -> Result<Option<BatchDetail>, String> {
    let created_at: Option<String> = conn
        .query_row(
            "SELECT created_at FROM job_batches WHERE id = ?1",
            params![batch_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    let Some(created_at) = created_at else {
        return Ok(None);
    };

    let mut stmt = conn
        .prepare(
            "SELECT j.id FROM transcription_jobs j WHERE j.batch_id = ?1 ORDER BY j.id ASC",
        )
        .map_err(|e| e.to_string())?;

    let ids: Vec<i64> = stmt
        .query_map(params![batch_id], |r| r.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut jobs = Vec::new();
    for id in ids {
        if let Some(detail) = get_job_detail(conn, id)? {
            jobs.push(detail);
        }
    }

    Ok(Some(BatchDetail {
        id: batch_id,
        created_at,
        jobs,
    }))
}

pub fn mark_job_processing(conn: &Connection, job_id: i64) -> Result<(), String> {
    let ts = now();
    conn.execute(
        "UPDATE transcription_jobs
         SET status = 'processing', started_at = ?1, updated_at = ?1, error_message = NULL
         WHERE id = ?2 AND status != 'completed'",
        params![ts, job_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn mark_job_completed(
    conn: &Connection,
    storage: &StorageRoot,
    job_id: i64,
    outcome: &TranscriptionOutcome,
) -> Result<(), String> {
    let detail = get_job_detail(conn, job_id)?
        .ok_or_else(|| format!("Job {job_id} not found"))?;

    let md = markdown::render(
        &outcome.text,
        &detail.original_filename,
        &detail.provider_key,
    );
    let md_path = storage.write_markdown(detail.upload_id, &md)?;
    let md_path_str = md_path.to_string_lossy().to_string();
    let meta = serde_json::to_string(&outcome.metadata).unwrap_or_else(|_| "{}".into());
    let ts = now();

    conn.execute(
        "INSERT INTO transcription_results (job_id, markdown_path, transcript_text, provider_metadata_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(job_id) DO UPDATE SET
           markdown_path = excluded.markdown_path,
           transcript_text = excluded.transcript_text,
           provider_metadata_json = excluded.provider_metadata_json",
        params![job_id, md_path_str, outcome.text, meta, ts],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE transcription_jobs
         SET status = 'completed', completed_at = ?1, updated_at = ?1, error_message = NULL
         WHERE id = ?2",
        params![ts, job_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn mark_job_failed(conn: &Connection, job_id: i64, message: &str) -> Result<(), String> {
    let ts = now();
    let trimmed: String = message.chars().take(500).collect();
    conn.execute(
        "UPDATE transcription_jobs
         SET status = 'failed', error_message = ?1, completed_at = ?2, updated_at = ?2
         WHERE id = ?3",
        params![trimmed, ts, job_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn retry_job(conn: &Connection, job_id: i64) -> Result<JobDetail, String> {
    let detail = get_job_detail(conn, job_id)?
        .ok_or_else(|| format!("Job {job_id} not found"))?;

    if !detail.retryable {
        return Err("Job is not retryable".into());
    }

    let ts = now();
    conn.execute(
        "UPDATE transcription_jobs
         SET status = 'queued', error_message = NULL, started_at = NULL, completed_at = NULL, updated_at = ?1
         WHERE id = ?2",
        params![ts, job_id],
    )
    .map_err(|e| e.to_string())?;

    // Remove previous result if any
    conn.execute(
        "DELETE FROM transcription_results WHERE job_id = ?1",
        params![job_id],
    )
    .map_err(|e| e.to_string())?;

    get_job_detail(conn, job_id)?.ok_or_else(|| "Job missing after retry".into())
}

fn job_audio_path(conn: &Connection, job_id: i64) -> Result<(String, String), String> {
    conn.query_row(
        "SELECT u.audio_path, j.provider_key
         FROM transcription_jobs j
         INNER JOIN uploads u ON u.id = j.upload_id
         WHERE j.id = ?1",
        params![job_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .map_err(|e| e.to_string())
}

/// Work package loaded under a short DB lock; safe to use after the lock is released.
#[derive(Debug, Clone)]
pub struct PreparedTranscription {
    pub audio_path: String,
    pub opts: TranscribeOptions,
}

/// Phase 1 (short lock): mark processing and load paths/settings. Returns `Ok(None)` if already completed.
pub fn prepare_transcription(
    conn: &Connection,
    job_id: i64,
) -> Result<Option<PreparedTranscription>, String> {
    let detail = get_job_detail(conn, job_id)?
        .ok_or_else(|| format!("Job {job_id} not found"))?;

    if detail.status == "completed" {
        return Ok(None);
    }

    mark_job_processing(conn, job_id)?;

    let (audio_path, provider_key) = job_audio_path(conn, job_id)?;
    if provider_key != "assemblyai" {
        let msg = format!("Provider {provider_key} is not available in this version");
        mark_job_failed(conn, job_id, &msg)?;
        return Err(msg);
    }

    let api_key = match settings::assemblyai_api_key(conn)? {
        Some(k) => k,
        None => {
            let msg = "AssemblyAI requires an API key in Settings".to_string();
            mark_job_failed(conn, job_id, &msg)?;
            return Err(msg);
        }
    };

    let settings = settings::get_settings(conn)?;
    let language = if settings.language == "auto" {
        None
    } else {
        Some(settings.language)
    };

    Ok(Some(PreparedTranscription {
        audio_path,
        opts: TranscribeOptions {
            api_key,
            language,
            poll_interval: std::time::Duration::from_secs(3),
            max_polls: 60,
        },
    }))
}

/// Phase 2 (no DB): network upload/poll only — must not hold SQLite locks.
pub fn execute_transcription(
    transport: &dyn HttpTransport,
    prepared: &PreparedTranscription,
) -> Result<TranscriptionOutcome, String> {
    assemblyai::transcribe(transport, Path::new(&prepared.audio_path), &prepared.opts)
}

/// Phase 3 (short lock): persist success or failure.
pub fn finish_transcription(
    conn: &Connection,
    storage: &StorageRoot,
    job_id: i64,
    result: Result<TranscriptionOutcome, String>,
) -> Result<(), String> {
    match result {
        Ok(outcome) => {
            mark_job_completed(conn, storage, job_id, &outcome)?;
            Ok(())
        }
        Err(e) => {
            mark_job_failed(conn, job_id, &e)?;
            Err(e)
        }
    }
}

/// Process one job: only holds `db` during prepare and finish — never during HTTP/poll.
///
/// This is the production entry path used by Tauri background workers.
pub fn process_transcription_job_with_lock(
    db: &parking_lot::Mutex<Connection>,
    storage: &StorageRoot,
    transport: &dyn HttpTransport,
    job_id: i64,
) -> Result<(), String> {
    let prepared = {
        let conn = db.lock();
        prepare_transcription(&conn, job_id)?
    };

    let Some(prepared) = prepared else {
        return Ok(());
    };

    // Network I/O + sleep: SQLite mutex is intentionally free so UI commands can run.
    let network_result = execute_transcription(transport, &prepared);

    {
        let conn = db.lock();
        finish_transcription(&conn, storage, job_id, network_result)
    }
}

/// Single-connection helper for unit tests that do not share a mutex.
#[cfg_attr(not(test), allow(dead_code))]
pub fn process_transcription_job(
    conn: &Connection,
    storage: &StorageRoot,
    transport: &dyn HttpTransport,
    job_id: i64,
) -> Result<(), String> {
    let prepared = prepare_transcription(conn, job_id)?;
    let Some(prepared) = prepared else {
        return Ok(());
    };
    let network_result = execute_transcription(transport, &prepared);
    finish_transcription(conn, storage, job_id, network_result)
}

pub fn read_transcript_markdown(conn: &Connection, job_id: i64) -> Result<String, String> {
    let detail = get_job_detail(conn, job_id)?
        .ok_or_else(|| format!("Job {job_id} not found"))?;
    let path = detail
        .markdown_path
        .ok_or_else(|| "Transcript not available".to_string())?;
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::domain::assemblyai::HttpTransport;
    use parking_lot::Mutex;
    use serde_json::{json, Value};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::{tempdir, NamedTempFile};

    struct SeqTransport {
        step: AtomicUsize,
        fail: bool,
    }

    impl HttpTransport for SeqTransport {
        fn post_bytes(
            &self,
            _url: &str,
            _headers: &[(&str, &str)],
            _body: Vec<u8>,
        ) -> Result<Value, String> {
            Ok(json!({"upload_url": "https://cdn.example/a.wav"}))
        }

        fn post_json(
            &self,
            _url: &str,
            _headers: &[(&str, &str)],
            _body: &Value,
        ) -> Result<Value, String> {
            Ok(json!({"id": "tr_1"}))
        }

        fn get_json(&self, _url: &str, _headers: &[(&str, &str)]) -> Result<Value, String> {
            let n = self.step.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                return Ok(json!({"status": "error", "error": "provider boom"}));
            }
            if n == 0 {
                Ok(json!({"status": "processing"}))
            } else {
                Ok(json!({
                    "status": "completed",
                    "text": "shipped domain transcript",
                    "id": "tr_1"
                }))
            }
        }
    }

    fn setup() -> (tempfile::TempDir, Connection, StorageRoot, NamedTempFile) {
        let dir = tempdir().unwrap();
        let conn = db::open(&dir.path().join("jobs.db")).unwrap();
        let storage = StorageRoot::new(dir.path().join("storage"));
        settings::update_settings(
            &conn,
            crate::domain::models::UpdateSettingsInput {
                workspace_name: "Local".into(),
                default_provider: "assemblyai".into(),
                assemblyai_api_key: Some("test-key".into()),
                language: "auto".into(),
            },
        )
        .unwrap();
        let audio = NamedTempFile::new().unwrap();
        std::fs::write(audio.path(), b"RIFF....WAVEfmt ").unwrap();
        (dir, conn, storage, audio)
    }

    #[test]
    fn lifecycle_create_complete_list_retry() {
        let (_dir, conn, storage, audio) = setup();
        let files = vec![NewUploadFile {
            filename: "clip.wav".into(),
            mime_type: "audio/wav".into(),
            size_bytes: 16,
            source_path: audio.path().to_string_lossy().into(),
        }];

        let created = create_uploads_and_jobs(&conn, &storage, &files).unwrap();
        assert_eq!(created.len(), 1);
        assert_eq!(created[0].status, "queued");
        assert!(created[0].batch_id.is_none());

        let job_id = created[0].id;

        // complete via mark helpers (simulating success)
        mark_job_processing(&conn, job_id).unwrap();
        mark_job_completed(
            &conn,
            &storage,
            job_id,
            &TranscriptionOutcome {
                text: "hello world".into(),
                metadata: json!({"id": "x"}),
            },
        )
        .unwrap();

        let detail = get_job_detail(&conn, job_id).unwrap().unwrap();
        assert_eq!(detail.status, "completed");
        assert_eq!(detail.transcript_text.as_deref(), Some("hello world"));
        assert!(detail.markdown_path.as_ref().unwrap().contains("transcript.md"));
        let md = std::fs::read_to_string(detail.markdown_path.as_ref().unwrap()).unwrap();
        assert!(md.contains("hello world"));
        assert!(md.contains("clip.wav"));

        let list = list_jobs(&conn).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].status, "completed");

        // fail + retry
        mark_job_failed(&conn, job_id, "forced fail").unwrap();
        let failed = get_job_detail(&conn, job_id).unwrap().unwrap();
        assert_eq!(failed.status, "failed");
        assert!(failed.retryable);

        let retried = retry_job(&conn, job_id).unwrap();
        assert_eq!(retried.status, "queued");
        assert!(retried.error_message.is_none());
    }

    #[test]
    fn list_job_rows_groups_batch_and_single() {
        let (_dir, conn, storage, audio) = setup();
        let path = audio.path().to_string_lossy().to_string();
        // batch of 2
        create_uploads_and_jobs(
            &conn,
            &storage,
            &[
                NewUploadFile {
                    filename: "a.wav".into(),
                    mime_type: "audio/wav".into(),
                    size_bytes: 10,
                    source_path: path.clone(),
                },
                NewUploadFile {
                    filename: "b.wav".into(),
                    mime_type: "audio/wav".into(),
                    size_bytes: 10,
                    source_path: path.clone(),
                },
            ],
        )
        .unwrap();
        // single
        create_uploads_and_jobs(
            &conn,
            &storage,
            &[NewUploadFile {
                filename: "solo.wav".into(),
                mime_type: "audio/wav".into(),
                size_bytes: 10,
                source_path: path,
            }],
        )
        .unwrap();

        let rows = list_job_rows(&conn).unwrap();
        assert_eq!(rows.len(), 2, "one batch row + one single row");
        let has_batch = rows.iter().any(|r| {
            matches!(
                r,
                crate::domain::grouping::JobListRow::Batch { jobs, .. } if jobs.len() == 2
            )
        });
        let has_single = rows.iter().any(|r| {
            matches!(
                r,
                crate::domain::grouping::JobListRow::Single { job } if job.original_filename == "solo.wav"
            )
        });
        assert!(has_batch);
        assert!(has_single);
    }

    #[test]
    fn rejects_unsupported_and_empty_uploads() {
        let (_dir, conn, storage, audio) = setup();
        let err = create_uploads_and_jobs(&conn, &storage, &[]).unwrap_err();
        assert!(err.to_lowercase().contains("no files"));

        let bad = NamedTempFile::new().unwrap();
        std::fs::write(bad.path(), b"x").unwrap();
        let err = create_uploads_and_jobs(
            &conn,
            &storage,
            &[NewUploadFile {
                filename: "notes.txt".into(),
                mime_type: "text/plain".into(),
                size_bytes: 1,
                source_path: bad.path().to_string_lossy().into(),
            }],
        )
        .unwrap_err();
        assert!(err.to_lowercase().contains("unsupported") || err.contains("notes.txt"));

        // missing source
        let err = create_uploads_and_jobs(
            &conn,
            &storage,
            &[NewUploadFile {
                filename: "gone.wav".into(),
                mime_type: "audio/wav".into(),
                size_bytes: 1,
                source_path: "/tmp/does-not-exist-mt-test.wav".into(),
            }],
        )
        .unwrap_err();
        assert!(err.to_lowercase().contains("missing") || err.contains("gone.wav"));
        let _ = audio;
    }

    #[test]
    fn multi_file_creates_batch() {
        let (_dir, conn, storage, audio) = setup();
        let path = audio.path().to_string_lossy().to_string();
        let files = vec![
            NewUploadFile {
                filename: "a.wav".into(),
                mime_type: "audio/wav".into(),
                size_bytes: 10,
                source_path: path.clone(),
            },
            NewUploadFile {
                filename: "b.wav".into(),
                mime_type: "audio/wav".into(),
                size_bytes: 10,
                source_path: path,
            },
        ];
        let created = create_uploads_and_jobs(&conn, &storage, &files).unwrap();
        assert_eq!(created.len(), 2);
        let batch_id = created[0].batch_id.unwrap();
        assert_eq!(created[1].batch_id, Some(batch_id));

        let batch = get_batch_detail(&conn, batch_id).unwrap().unwrap();
        assert_eq!(batch.jobs.len(), 2);
    }

    #[test]
    fn process_job_success_and_failure_via_transport() {
        let (_dir, conn, storage, audio) = setup();
        let files = vec![NewUploadFile {
            filename: "talk.wav".into(),
            mime_type: "audio/wav".into(),
            size_bytes: 16,
            source_path: audio.path().to_string_lossy().into(),
        }];
        let job_id = create_uploads_and_jobs(&conn, &storage, &files).unwrap()[0].id;

        let ok = SeqTransport {
            step: AtomicUsize::new(0),
            fail: false,
        };
        // Drive real prepare → execute → finish with zero poll (override opts after prepare).
        let mut prepared = prepare_transcription(&conn, job_id).unwrap().unwrap();
        prepared.opts.poll_interval = std::time::Duration::ZERO;
        let outcome = execute_transcription(&ok, &prepared).unwrap();
        finish_transcription(&conn, &storage, job_id, Ok(outcome)).unwrap();
        let detail = get_job_detail(&conn, job_id).unwrap().unwrap();
        assert_eq!(detail.status, "completed");
        assert_eq!(
            detail.transcript_text.as_deref(),
            Some("shipped domain transcript")
        );

        // failure path via full process_transcription_job entry
        let files2 = vec![NewUploadFile {
            filename: "bad.wav".into(),
            mime_type: "audio/wav".into(),
            size_bytes: 16,
            source_path: audio.path().to_string_lossy().into(),
        }];
        let job2 = create_uploads_and_jobs(&conn, &storage, &files2).unwrap()[0].id;
        let fail_t = SeqTransport {
            step: AtomicUsize::new(0),
            fail: true,
        };
        let err = process_transcription_job(&conn, &storage, &fail_t, job2).unwrap_err();
        assert_eq!(err, "provider boom");
        let failed = get_job_detail(&conn, job2).unwrap().unwrap();
        assert_eq!(failed.status, "failed");
        assert_eq!(failed.error_message.as_deref(), Some("provider boom"));
    }

    /// Regression: app-wide Mutex must not be held during transport I/O, or UI list/get freeze.
    #[test]
    fn process_with_lock_releases_db_during_network() {
        use std::sync::Arc;
        use std::thread;
        use std::time::{Duration, Instant};

        struct BlockingTransport {
            network_entered: Arc<std::sync::Barrier>,
            list_done: Arc<std::sync::Barrier>,
        }

        impl HttpTransport for BlockingTransport {
            fn post_bytes(
                &self,
                _url: &str,
                _headers: &[(&str, &str)],
                _body: Vec<u8>,
            ) -> Result<Value, String> {
                // Network phase started with no DB lock held; wait for concurrent list_jobs.
                self.network_entered.wait();
                self.list_done.wait();
                Ok(json!({"upload_url": "https://cdn.example/a.wav"}))
            }

            fn post_json(
                &self,
                _url: &str,
                _headers: &[(&str, &str)],
                _body: &Value,
            ) -> Result<Value, String> {
                Ok(json!({"id": "tr_lock"}))
            }

            fn get_json(&self, _url: &str, _headers: &[(&str, &str)]) -> Result<Value, String> {
                Ok(json!({
                    "status": "completed",
                    "text": "unlocked transcript",
                    "id": "tr_lock"
                }))
            }
        }

        let dir = tempdir().unwrap();
        let conn = db::open(&dir.path().join("lock.db")).unwrap();
        let storage = StorageRoot::new(dir.path().join("storage"));
        settings::update_settings(
            &conn,
            crate::domain::models::UpdateSettingsInput {
                workspace_name: "Local".into(),
                default_provider: "assemblyai".into(),
                assemblyai_api_key: Some("test-key".into()),
                language: "auto".into(),
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

        // Same Mutex shape as AppState / production path.
        drop(conn);
        let db = Arc::new(Mutex::new(db::open(&dir.path().join("lock.db")).unwrap()));
        let network_entered = Arc::new(std::sync::Barrier::new(2));
        let list_done = Arc::new(std::sync::Barrier::new(2));
        let transport = BlockingTransport {
            network_entered: network_entered.clone(),
            list_done: list_done.clone(),
        };

        let db_worker = db.clone();
        let storage_worker = StorageRoot::new(dir.path().join("storage"));
        // Production entry used by Tauri spawn_process_job.
        let worker = thread::spawn(move || {
            process_transcription_job_with_lock(&db_worker, &storage_worker, &transport, job_id)
        });

        // Concurrent UI-style read while worker is blocked in network phase.
        network_entered.wait();
        let start = Instant::now();
        {
            let conn = db.lock();
            let list = list_jobs(&conn).unwrap();
            assert_eq!(list.len(), 1);
            assert_eq!(list[0].status, "processing");
        }
        let elapsed = start.elapsed();
        list_done.wait();
        assert!(
            elapsed < Duration::from_millis(500),
            "list_jobs blocked too long ({elapsed:?}) — DB lock likely held during network"
        );

        worker.join().unwrap().unwrap();
        let conn = db.lock();
        let done = get_job_detail(&conn, job_id).unwrap().unwrap();
        assert_eq!(done.status, "completed");
        assert_eq!(
            done.transcript_text.as_deref(),
            Some("unlocked transcript")
        );
    }

    #[test]
    fn prepare_skips_completed_and_fails_without_api_key() {
        let (_dir, conn, storage, audio) = setup();
        let path = audio.path().to_string_lossy().to_string();
        let job_id = create_uploads_and_jobs(
            &conn,
            &storage,
            &[NewUploadFile {
                filename: "c.wav".into(),
                mime_type: "audio/wav".into(),
                size_bytes: 8,
                source_path: path.clone(),
            }],
        )
        .unwrap()[0]
        .id;

        mark_job_processing(&conn, job_id).unwrap();
        mark_job_completed(
            &conn,
            &storage,
            job_id,
            &TranscriptionOutcome {
                text: "done".into(),
                metadata: json!({}),
            },
        )
        .unwrap();
        assert!(prepare_transcription(&conn, job_id).unwrap().is_none());

        // clear api key, new job → prepare fails and marks failed
        settings::update_settings(
            &conn,
            crate::domain::models::UpdateSettingsInput {
                workspace_name: "Local".into(),
                default_provider: "assemblyai".into(),
                assemblyai_api_key: Some("".into()),
                language: "auto".into(),
            },
        )
        .unwrap();
        let job2 = create_uploads_and_jobs(
            &conn,
            &storage,
            &[NewUploadFile {
                filename: "n.wav".into(),
                mime_type: "audio/wav".into(),
                size_bytes: 8,
                source_path: path,
            }],
        )
        .unwrap()[0]
        .id;
        let err = prepare_transcription(&conn, job2).unwrap_err();
        assert!(err.to_lowercase().contains("api key"));
        let detail = get_job_detail(&conn, job2).unwrap().unwrap();
        assert_eq!(detail.status, "failed");
    }

    #[test]
    fn read_transcript_and_reject_non_retryable() {
        let (_dir, conn, storage, audio) = setup();
        let job_id = create_uploads_and_jobs(
            &conn,
            &storage,
            &[NewUploadFile {
                filename: "r.wav".into(),
                mime_type: "audio/wav".into(),
                size_bytes: 8,
                source_path: audio.path().to_string_lossy().into(),
            }],
        )
        .unwrap()[0]
        .id;

        mark_job_processing(&conn, job_id).unwrap();
        mark_job_completed(
            &conn,
            &storage,
            job_id,
            &TranscriptionOutcome {
                text: "read me".into(),
                metadata: json!({}),
            },
        )
        .unwrap();

        let md = read_transcript_markdown(&conn, job_id).unwrap();
        assert!(md.contains("read me"));
        assert!(md.contains("r.wav"));

        let err = retry_job(&conn, job_id).unwrap_err();
        assert!(err.to_lowercase().contains("not retryable"));
    }

    #[test]
    fn process_transcription_job_with_lock_is_production_entry() {
        let dir = tempdir().unwrap();
        let conn = db::open(&dir.path().join("prod.db")).unwrap();
        let storage = StorageRoot::new(dir.path().join("storage"));
        settings::update_settings(
            &conn,
            crate::domain::models::UpdateSettingsInput {
                workspace_name: "Local".into(),
                default_provider: "assemblyai".into(),
                assemblyai_api_key: Some("k".into()),
                language: "en".into(),
            },
        )
        .unwrap();
        let audio = NamedTempFile::new().unwrap();
        std::fs::write(audio.path(), b"audio").unwrap();
        let job_id = create_uploads_and_jobs(
            &conn,
            &storage,
            &[NewUploadFile {
                filename: "p.wav".into(),
                mime_type: "audio/wav".into(),
                size_bytes: 5,
                source_path: audio.path().to_string_lossy().into(),
            }],
        )
        .unwrap()[0]
        .id;
        drop(conn);

        let db = Mutex::new(db::open(&dir.path().join("prod.db")).unwrap());
        // Fail fast via transport error — still exercises with_lock path end-to-end.
        let fail_t = SeqTransport {
            step: AtomicUsize::new(0),
            fail: true,
        };
        let err =
            process_transcription_job_with_lock(&db, &storage, &fail_t, job_id).unwrap_err();
        assert_eq!(err, "provider boom");
        let conn = db.lock();
        assert_eq!(
            get_job_detail(&conn, job_id).unwrap().unwrap().status,
            "failed"
        );
    }
}
