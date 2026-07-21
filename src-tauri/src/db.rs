//! SQLite connection and schema (single-workspace, no users/auth).

use rusqlite::{Connection, OptionalExtension};
use std::path::Path;

pub fn open(db_path: &Path) -> Result<Connection, String> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute_batch("PRAGMA foreign_keys = ON;").map_err(|e| e.to_string())?;
    migrate(&conn)?;
    Ok(conn)
}

pub fn migrate(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS app_settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            workspace_name TEXT NOT NULL DEFAULT 'Local',
            default_provider TEXT NOT NULL DEFAULT 'assemblyai',
            assemblyai_api_key TEXT,
            language TEXT NOT NULL DEFAULT 'auto',
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS uploads (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            original_filename TEXT NOT NULL,
            mime_type TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            audio_path TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS job_batches (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS transcription_jobs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            upload_id INTEGER NOT NULL UNIQUE REFERENCES uploads(id) ON DELETE CASCADE,
            batch_id INTEGER REFERENCES job_batches(id) ON DELETE SET NULL,
            provider_key TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'queued',
            error_message TEXT,
            started_at TEXT,
            completed_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_jobs_status ON transcription_jobs(status);
        CREATE INDEX IF NOT EXISTS idx_jobs_batch ON transcription_jobs(batch_id);

        CREATE TABLE IF NOT EXISTS transcription_results (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            job_id INTEGER NOT NULL UNIQUE REFERENCES transcription_jobs(id) ON DELETE CASCADE,
            markdown_path TEXT NOT NULL,
            transcript_text TEXT NOT NULL,
            provider_metadata_json TEXT,
            created_at TEXT NOT NULL
        );
        "#,
    )
    .map_err(|e| e.to_string())?;

    // Seed single settings row if missing
    let exists: Option<i64> = conn
        .query_row("SELECT id FROM app_settings WHERE id = 1", [], |r| r.get(0))
        .optional()
        .map_err(|e| e.to_string())?;

    if exists.is_none() {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO app_settings (id, workspace_name, default_provider, assemblyai_api_key, language, updated_at)
             VALUES (1, 'Local', 'assemblyai', NULL, 'auto', ?1)",
            [&now],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn migrate_creates_tables_without_auth() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let conn = open(&path).unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();

        assert!(tables.contains(&"app_settings".into()));
        assert!(tables.contains(&"uploads".into()));
        assert!(tables.contains(&"job_batches".into()));
        assert!(tables.contains(&"transcription_jobs".into()));
        assert!(tables.contains(&"transcription_results".into()));
        assert!(!tables.iter().any(|t| t.contains("user")));
        assert!(!tables.iter().any(|t| t.contains("tenant")));
        assert!(!tables.iter().any(|t| t.contains("session")));
        assert!(!tables.iter().any(|t| t.contains("password")));
    }
}
