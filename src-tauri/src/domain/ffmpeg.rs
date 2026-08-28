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
