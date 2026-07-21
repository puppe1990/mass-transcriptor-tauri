//! Shared app paths and SQLite access for Tauri commands.

use crate::db;
use crate::domain::storage::StorageRoot;
use parking_lot::Mutex;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db_path: PathBuf,
    pub storage_root: PathBuf,
    /// Serializes SQLite access (rusqlite Connection is not Sync across threads easily).
    pub db: Arc<Mutex<Connection>>,
}

impl AppState {
    pub fn initialize(app_data_dir: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;
        let db_path = app_data_dir.join("mass-transcriptor.db");
        let storage_root = app_data_dir.join("storage");
        std::fs::create_dir_all(&storage_root).map_err(|e| e.to_string())?;

        let conn = db::open(&db_path)?;
        Ok(Self {
            db_path,
            storage_root,
            db: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn storage(&self) -> StorageRoot {
        StorageRoot::new(self.storage_root.clone())
    }
}
