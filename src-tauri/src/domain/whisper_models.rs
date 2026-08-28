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
        let mut resp = self.client.get(url).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }
        let mut file = std::fs::File::create(dest).map_err(|e| e.to_string())?;
        std::io::copy(&mut resp, &mut file).map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub fn sha1_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha1::digest(bytes))
}

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
