//! Download and verify ggml Whisper weights. Jobs never download; Settings does.

use serde::Serialize;
use sha1::{Digest, Sha1};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhisperModel {
    Tiny,
    Base,
    Small,
    MediumQ8,
    LargeV3TurboQ8,
    LargeV3Q5,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WhisperModelStatus {
    pub id: String,
    pub display_name: String,
    pub size_label: String,
    pub size_bytes: u64,
    pub quality_hint: String,
    pub filename: String,
    pub installed: bool,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequireModelError {
    Missing,
    Corrupt,
    Io(String),
}

impl WhisperModel {
    pub const ALL: [Self; 6] = [
        Self::Tiny,
        Self::Base,
        Self::Small,
        Self::MediumQ8,
        Self::LargeV3TurboQ8,
        Self::LargeV3Q5,
    ];

    pub fn parse(s: &str) -> Result<Self, String> {
        match s.trim().to_lowercase().as_str() {
            "tiny" => Ok(Self::Tiny),
            "base" => Ok(Self::Base),
            "small" => Ok(Self::Small),
            "medium" | "medium-q8" | "medium-q8_0" => Ok(Self::MediumQ8),
            "turbo" | "large-v3-turbo" | "large-v3-turbo-q8" | "large-v3-turbo-q8_0" => {
                Ok(Self::LargeV3TurboQ8)
            }
            "large" | "large-v3" | "large-v3-q5" | "large-v3-q5_0" => Ok(Self::LargeV3Q5),
            other => Err(format!("Whisper model '{other}' is not allowed")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tiny => "tiny",
            Self::Base => "base",
            Self::Small => "small",
            Self::MediumQ8 => "medium-q8",
            Self::LargeV3TurboQ8 => "large-v3-turbo-q8",
            Self::LargeV3Q5 => "large-v3-q5",
        }
    }

    pub fn filename(self) -> &'static str {
        match self {
            Self::Tiny => "ggml-tiny.bin",
            Self::Base => "ggml-base.bin",
            Self::Small => "ggml-small.bin",
            Self::MediumQ8 => "ggml-medium-q8_0.bin",
            Self::LargeV3TurboQ8 => "ggml-large-v3-turbo-q8_0.bin",
            Self::LargeV3Q5 => "ggml-large-v3-q5_0.bin",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Tiny => "Tiny",
            Self::Base => "Base",
            Self::Small => "Small",
            Self::MediumQ8 => "Medium Q8",
            Self::LargeV3TurboQ8 => "Large v3 Turbo Q8",
            Self::LargeV3Q5 => "Large v3 Q5",
        }
    }

    pub fn quality_hint(self) -> &'static str {
        match self {
            Self::Tiny => "Fastest. Rough draft.",
            Self::Base => "Fast. Short clips.",
            Self::Small => "Balanced. Good everyday pick.",
            Self::MediumQ8 => "Slower. Better names and numbers.",
            Self::LargeV3TurboQ8 => "Strong multilingual, closer to large speed.",
            Self::LargeV3Q5 => "Highest accuracy. Slowest and largest.",
        }
    }

    pub fn url(self) -> String {
        format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}",
            self.filename()
        )
    }

    /// SHA1 pins from https://huggingface.co/ggerganov/whisper.cpp
    pub fn sha1_hex(self) -> &'static str {
        match self {
            Self::Tiny => "bd577a113a864445d4c299885e0cb97d4ba92b5f",
            Self::Base => "465707469ff3a37a2b9b8d8f89f2f99de7299dac",
            Self::Small => "55356645c2b361a969dfd0ef2c5a50d530afd8d5",
            Self::MediumQ8 => "e66645948aff4bebbec71b3485c576f3d63af5d6",
            Self::LargeV3TurboQ8 => "01bf15bedffe9f39d65c1b6ff9b687ea91f59e0e",
            Self::LargeV3Q5 => "e6e2ed78495d403bef4b7cff42ef4aaadcfea8de",
        }
    }

    pub fn size_label(self) -> &'static str {
        match self {
            Self::Tiny => "~75 MB",
            Self::Base => "~142 MB",
            Self::Small => "~466 MB",
            Self::MediumQ8 => "~785 MB",
            Self::LargeV3TurboQ8 => "~834 MB",
            Self::LargeV3Q5 => "~1.1 GB",
        }
    }

    pub fn size_bytes(self) -> u64 {
        match self {
            Self::Tiny => 75 * 1024 * 1024,
            Self::Base => 142 * 1024 * 1024,
            Self::Small => 466 * 1024 * 1024,
            Self::MediumQ8 => 785 * 1024 * 1024,
            Self::LargeV3TurboQ8 => 834 * 1024 * 1024,
            Self::LargeV3Q5 => (11 * 1024 * 1024 * 1024) / 10,
        }
    }

    pub fn is_installed(self, models_dir: &Path) -> bool {
        models_dir.join(self.filename()).is_file()
    }

    pub fn status(self, models_dir: &Path, selected: &str) -> WhisperModelStatus {
        let selected_id = Self::parse(selected).ok();
        WhisperModelStatus {
            id: self.as_str().to_string(),
            display_name: self.display_name().to_string(),
            size_label: self.size_label().to_string(),
            size_bytes: self.size_bytes(),
            quality_hint: self.quality_hint().to_string(),
            filename: self.filename().to_string(),
            installed: self.is_installed(models_dir),
            selected: selected_id == Some(self),
        }
    }
}

/// Persist a model choice only if the ggml file is already on disk.
/// AssemblyAI may keep an existing undownloaded id so other settings still save.
pub fn assert_can_persist_model(
    model: WhisperModel,
    provider: &str,
    existing_model: &str,
    models_dir: &Path,
) -> Result<(), String> {
    if model.is_installed(models_dir) {
        return Ok(());
    }
    let keeping_existing = model.as_str() == existing_model;
    if provider != "whisper" && keeping_existing {
        return Ok(());
    }
    Err(format!(
        "Download Whisper model `{}` ({}) before selecting it. It cannot be saved until the file is on disk.",
        model.as_str(),
        model.size_label()
    ))
}

pub fn list_status(models_dir: &Path, selected: &str) -> Vec<WhisperModelStatus> {
    WhisperModel::ALL
        .into_iter()
        .map(|m| m.status(models_dir, selected))
        .collect()
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
                .timeout(std::time::Duration::from_secs(7200))
                .user_agent("mass-transcriptor-tauri")
                .build()
                .expect("reqwest client"),
        }
    }
}

impl ReqwestModelDownloader {
    pub fn download_with_progress(
        &self,
        url: &str,
        dest: &Path,
        on_progress: &dyn Fn(u64, Option<u64>),
    ) -> Result<(), String> {
        let mut resp = self.client.get(url).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }
        let total = resp.content_length();
        let mut file = std::fs::File::create(dest).map_err(|e| e.to_string())?;
        let mut buf = [0u8; 64 * 1024];
        let mut downloaded = 0u64;
        loop {
            let n = resp.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
            downloaded += n as u64;
            on_progress(downloaded, total);
        }
        Ok(())
    }
}

impl ModelDownloader for ReqwestModelDownloader {
    fn download(&self, url: &str, dest: &Path) -> Result<(), String> {
        self.download_with_progress(url, dest, &|_, _| {})
    }
}

pub fn sha1_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha1::digest(bytes))
}

pub fn sha1_file(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha1::new();
    let mut buf = [0u8; 1024 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn verify_hash(path: &Path, expected_sha1: &str) -> Result<(), RequireModelError> {
    match sha1_file(path) {
        Ok(got) if got == expected_sha1 => Ok(()),
        Ok(_) => Err(RequireModelError::Corrupt),
        Err(e) => Err(RequireModelError::Io(e)),
    }
}

/// Jobs use this. Never downloads.
pub fn require_model(models_dir: &Path, model: WhisperModel) -> Result<PathBuf, RequireModelError> {
    let target = models_dir.join(model.filename());
    if !target.exists() {
        return Err(RequireModelError::Missing);
    }
    #[cfg(not(test))]
    {
        verify_hash(&target, model.sha1_hex())?;
    }
    Ok(target)
}

/// Explicit Settings download. Jobs must not call this.
pub fn download_model(
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

pub fn download_model_with_progress(
    models_dir: &Path,
    model: WhisperModel,
    downloader: &ReqwestModelDownloader,
    on_progress: &dyn Fn(u64, Option<u64>),
) -> Result<PathBuf, String> {
    install_artifact(models_dir, model.filename(), model.sha1_hex(), |partial| {
        downloader.download_with_progress(&model.url(), partial, on_progress)
    })
}

pub fn delete_model(models_dir: &Path, model: WhisperModel) -> Result<(), String> {
    let target = models_dir.join(model.filename());
    let partial = models_dir.join(format!("{}.partial", model.filename()));
    let _ = std::fs::remove_file(&partial);
    if target.exists() {
        std::fs::remove_file(&target).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn ensure_artifact(
    models_dir: &Path,
    filename: &str,
    url: &str,
    expected_sha1: &str,
    downloader: &dyn ModelDownloader,
) -> Result<PathBuf, String> {
    install_artifact(models_dir, filename, expected_sha1, |partial| {
        downloader.download(url, partial)
    })
}

fn install_artifact(
    models_dir: &Path,
    filename: &str,
    expected_sha1: &str,
    download: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(models_dir).map_err(|e| e.to_string())?;
    let target = models_dir.join(filename);
    if target.exists() {
        let got = sha1_file(&target)?;
        if got == expected_sha1 {
            return Ok(target);
        }
        std::fs::remove_file(&target).map_err(|e| e.to_string())?;
    }

    let partial = models_dir.join(format!("{filename}.partial"));
    let _ = std::fs::remove_file(&partial);
    if let Err(e) = download(&partial) {
        let _ = std::fs::remove_file(&partial);
        return Err(e);
    }
    let got = match sha1_file(&partial) {
        Ok(h) => h,
        Err(e) => {
            let _ = std::fs::remove_file(&partial);
            return Err(e);
        }
    };
    if got != expected_sha1 {
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
        assert_eq!(
            WhisperModel::parse("medium").unwrap(),
            WhisperModel::MediumQ8
        );
        assert_eq!(
            WhisperModel::parse("turbo").unwrap(),
            WhisperModel::LargeV3TurboQ8
        );
        assert_eq!(
            WhisperModel::parse("large").unwrap(),
            WhisperModel::LargeV3Q5
        );
        assert!(WhisperModel::parse("parakeet")
            .unwrap_err()
            .contains("parakeet"));
        assert_eq!(WhisperModel::Tiny.filename(), "ggml-tiny.bin");
        assert_eq!(
            WhisperModel::Tiny.sha1_hex(),
            "bd577a113a864445d4c299885e0cb97d4ba92b5f"
        );
        assert_eq!(
            WhisperModel::MediumQ8.sha1_hex(),
            "e66645948aff4bebbec71b3485c576f3d63af5d6"
        );
        assert_eq!(
            WhisperModel::LargeV3TurboQ8.filename(),
            "ggml-large-v3-turbo-q8_0.bin"
        );
        assert_eq!(WhisperModel::LargeV3Q5.filename(), "ggml-large-v3-q5_0.bin");
    }

    #[test]
    fn persist_rejects_missing_model_for_whisper() {
        let dir = tempdir().unwrap();
        let err = assert_can_persist_model(WhisperModel::Base, "whisper", "base", dir.path())
            .unwrap_err();
        assert!(err.contains("before selecting"));
        assert!(err.contains("`base`"));
    }

    #[test]
    fn persist_allows_assemblyai_to_keep_existing_missing_model() {
        let dir = tempdir().unwrap();
        assert_can_persist_model(WhisperModel::Base, "assemblyai", "base", dir.path()).unwrap();
    }

    #[test]
    fn persist_rejects_switching_to_another_missing_model() {
        let dir = tempdir().unwrap();
        let err = assert_can_persist_model(WhisperModel::Small, "assemblyai", "base", dir.path())
            .unwrap_err();
        assert!(err.contains("`small`"));
    }

    #[test]
    fn persist_allows_installed_model() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("ggml-tiny.bin"), b"stub").unwrap();
        assert_can_persist_model(WhisperModel::Tiny, "whisper", "base", dir.path()).unwrap();
    }

    #[test]
    fn catalog_lists_all_models_without_downloading() {
        let dir = tempdir().unwrap();
        let list = list_status(dir.path(), "base");
        assert_eq!(list.len(), 6);
        assert!(list.iter().all(|m| !m.installed));
        assert_eq!(list.iter().filter(|m| m.selected).count(), 1);
        assert_eq!(list[1].id, "base");
        assert!(list[1].selected);
        assert_eq!(list[3].id, "medium-q8");
        assert_eq!(list[4].id, "large-v3-turbo-q8");
        assert_eq!(list[5].id, "large-v3-q5");
    }

    #[test]
    fn require_model_missing_does_not_create_files() {
        let dir = tempdir().unwrap();
        let err = require_model(dir.path(), WhisperModel::Base).unwrap_err();
        assert_eq!(err, RequireModelError::Missing);
        assert!(std::fs::read_dir(dir.path()).unwrap().next().is_none());
    }

    #[test]
    fn require_model_accepts_existing_stub_in_tests() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("ggml-small.bin"), b"stub").unwrap();
        let path = require_model(dir.path(), WhisperModel::Small).unwrap();
        assert!(path.ends_with("ggml-small.bin"));
    }

    #[test]
    fn delete_model_removes_file_and_partial() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("ggml-base.bin"), b"x").unwrap();
        std::fs::write(dir.path().join("ggml-base.bin.partial"), b"y").unwrap();
        delete_model(dir.path(), WhisperModel::Base).unwrap();
        assert!(!dir.path().join("ggml-base.bin").exists());
        assert!(!dir.path().join("ggml-base.bin.partial").exists());
    }

    #[test]
    fn list_status_marks_installed_when_file_exists() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("ggml-tiny.bin"), b"x").unwrap();
        let list = list_status(dir.path(), "tiny");
        let tiny = list.iter().find(|m| m.id == "tiny").unwrap();
        assert!(tiny.installed);
        assert!(tiny.selected);
        assert!(!list.iter().find(|m| m.id == "base").unwrap().installed);
    }

    #[test]
    fn download_model_is_explicit_and_pins_sha1() {
        let dir = tempdir().unwrap();
        let dl = CountingDownloader {
            payload: b"not-the-real-weights".to_vec(),
            calls: AtomicUsize::new(0),
            fail: false,
        };
        let err = download_model(dir.path(), WhisperModel::Tiny, &dl).unwrap_err();
        assert!(err.contains("sha1"));
        assert_eq!(dl.calls.load(Ordering::SeqCst), 1);
        assert!(!dir.path().join("ggml-tiny.bin").exists());
    }

    #[test]
    fn verify_hash_detects_corrupt_and_io() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ggml-base.bin");
        std::fs::write(&path, b"nope").unwrap();
        assert_eq!(
            verify_hash(&path, WhisperModel::Base.sha1_hex()),
            Err(RequireModelError::Corrupt)
        );
        assert!(matches!(
            verify_hash(&dir.path().join("missing.bin"), "00"),
            Err(RequireModelError::Io(_))
        ));
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
        let path =
            ensure_artifact(dir.path(), "ggml-base.bin", "http://x", &expected, &dl).unwrap();
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
        let path =
            ensure_artifact(dir.path(), "ggml-tiny.bin", "http://x", &expected, &dl).unwrap();
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
        let err =
            ensure_artifact(dir.path(), "ggml-small.bin", "http://x", "deadbeef", &dl).unwrap_err();
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
        let path =
            ensure_artifact(dir.path(), "ggml-base.bin", "http://x", &expected, &dl).unwrap();
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
