//! Convert source media to 16 kHz mono WAV for whisper.cpp.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub trait Ffmpeg: Send + Sync {
    fn is_available(&self) -> bool;
    fn to_wav_16k_mono(&self, src: &Path, dest: &Path) -> Result<(), String>;
}

/// Homebrew and other installs that GUI apps miss because Finder PATH
/// is typically `/usr/bin:/bin` without `/opt/homebrew/bin`.
const FALLBACK_FFMPEG: &[&str] = &[
    "/opt/homebrew/bin/ffmpeg",
    "/usr/local/bin/ffmpeg",
    "/opt/homebrew/opt/ffmpeg/bin/ffmpeg",
];

fn ffmpeg_bin_name() -> &'static str {
    if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    }
}

fn is_runnable_file(path: &Path) -> bool {
    path.is_file()
}

/// Look up ffmpeg on PATH first, then well-known absolute locations.
pub fn resolve_ffmpeg_bin() -> Option<PathBuf> {
    resolve_ffmpeg_bin_with(
        std::env::var_os("PATH"),
        &FALLBACK_FFMPEG.iter().map(Path::new).collect::<Vec<_>>(),
    )
}

pub(crate) fn resolve_ffmpeg_bin_with(path_env: Option<OsString>, extras: &[&Path]) -> Option<PathBuf> {
    let name = ffmpeg_bin_name();
    if let Some(path) = path_env {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join(name);
            if is_runnable_file(&candidate) {
                return Some(candidate);
            }
        }
    }
    extras
        .iter()
        .find(|p| is_runnable_file(p))
        .map(|p| p.to_path_buf())
}

fn ffmpeg_command() -> Result<Command, String> {
    let bin = resolve_ffmpeg_bin().ok_or_else(|| "ffmpeg not found".to_string())?;
    Ok(Command::new(bin))
}

pub struct SystemFfmpeg;

impl Ffmpeg for SystemFfmpeg {
    fn is_available(&self) -> bool {
        let Ok(mut cmd) = ffmpeg_command() else {
            return false;
        };
        cmd.arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn to_wav_16k_mono(&self, src: &Path, dest: &Path) -> Result<(), String> {
        let output = ffmpeg_command()?
            .args(["-y", "-i"])
            .arg(src)
            .args(["-ac", "1", "-ar", "16000", "-c:a", "pcm_s16le"])
            .arg(dest)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
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

    #[test]
    fn finds_ffmpeg_in_fallback_when_path_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let ffmpeg = dir.path().join("ffmpeg");
        std::fs::write(&ffmpeg, b"#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut p = std::fs::metadata(&ffmpeg).unwrap().permissions();
            p.set_mode(0o755);
            std::fs::set_permissions(&ffmpeg, p).unwrap();
        }
        let found = resolve_ffmpeg_bin_with(None, &[ffmpeg.as_path()]).unwrap();
        assert_eq!(found, ffmpeg);
    }

    #[test]
    fn prefers_path_over_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let path_dir = dir.path().join("bin");
        std::fs::create_dir(&path_dir).unwrap();
        let on_path = path_dir.join("ffmpeg");
        std::fs::write(&on_path, b"path").unwrap();
        let fallback = dir.path().join("other-ffmpeg");
        std::fs::write(&fallback, b"fallback").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for f in [&on_path, &fallback] {
                let mut p = std::fs::metadata(f).unwrap().permissions();
                p.set_mode(0o755);
                std::fs::set_permissions(f, p).unwrap();
            }
        }
        let found = resolve_ffmpeg_bin_with(
            Some(path_dir.as_os_str().to_os_string()),
            &[fallback.as_path()],
        )
        .unwrap();
        assert_eq!(found, on_path);
    }

    #[test]
    fn missing_everywhere_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope").join("ffmpeg");
        assert!(resolve_ffmpeg_bin_with(None, &[missing.as_path()]).is_none());
    }
}
