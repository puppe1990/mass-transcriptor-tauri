//! Shared app paths and SQLite access for Tauri commands.

use crate::db;
use crate::domain::storage::StorageRoot;
use parking_lot::Mutex;
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db_path: PathBuf,
    pub storage_root: PathBuf,
    pub models_dir: PathBuf,
    /// Serializes SQLite access (rusqlite Connection is not Sync across threads easily).
    pub db: Arc<Mutex<Connection>>,
    cancels: Arc<Mutex<HashMap<i64, Arc<AtomicBool>>>>,
}

impl AppState {
    pub fn initialize(app_data_dir: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;
        let db_path = app_data_dir.join("mass-transcriptor.db");
        let storage_root = app_data_dir.join("storage");
        std::fs::create_dir_all(&storage_root).map_err(|e| e.to_string())?;
        let models_dir = app_data_dir.join("whisper-models");
        std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;

        let conn = db::open(&db_path)?;
        Ok(Self {
            db_path,
            storage_root,
            models_dir,
            db: Arc::new(Mutex::new(conn)),
            cancels: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn storage(&self) -> StorageRoot {
        StorageRoot::new(self.storage_root.clone())
    }

    pub fn cancel_flag(&self, job_id: i64) -> Arc<AtomicBool> {
        self.cancels
            .lock()
            .entry(job_id)
            .or_insert_with(|| Arc::new(AtomicBool::new(false)))
            .clone()
    }

    pub fn request_cancel(&self, job_id: i64) {
        self.cancel_flag(job_id).store(true, Ordering::SeqCst);
    }

    pub fn clear_cancel(&self, job_id: i64) {
        self.cancels.lock().remove(&job_id);
    }
}
