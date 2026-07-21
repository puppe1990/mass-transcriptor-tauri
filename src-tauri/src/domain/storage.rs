//! Local media / transcript paths under app data (no tenant slug).

use std::fs;
use std::path::{Path, PathBuf};

pub struct StorageRoot {
    root: PathBuf,
}

impl StorageRoot {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn upload_dir(&self, upload_id: i64) -> PathBuf {
        self.root
            .join("uploads")
            .join(upload_id.to_string())
    }

    pub fn audio_path(&self, upload_id: i64, filename: &str) -> PathBuf {
        self.upload_dir(upload_id).join("audio").join(filename)
    }

    pub fn markdown_path(&self, upload_id: i64) -> PathBuf {
        self.upload_dir(upload_id)
            .join("transcript")
            .join("transcript.md")
    }

    pub fn write_audio_from_path(
        &self,
        upload_id: i64,
        filename: &str,
        source: &Path,
    ) -> Result<PathBuf, String> {
        let dest = self.audio_path(upload_id, filename);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::copy(source, &dest).map_err(|e| e.to_string())?;
        Ok(dest)
    }

    pub fn write_markdown(&self, upload_id: i64, content: &str) -> Result<PathBuf, String> {
        let dest = self.markdown_path(upload_id);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&dest, content).map_err(|e| e.to_string())?;
        Ok(dest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn write_markdown_roundtrip() {
        let dir = tempdir().unwrap();
        let storage = StorageRoot::new(dir.path());
        let path = storage.write_markdown(7, "# hi\n").unwrap();
        assert!(path.exists());
        assert_eq!(fs::read_to_string(path).unwrap(), "# hi\n");
    }
}
