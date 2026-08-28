//! Local Whisper transcription (engine injected; production impl is whisper-rs).

use crate::domain::models::TranscriptionOutcome;
use crate::domain::whisper_models::WhisperModel;
use std::path::{Path, PathBuf};

pub const ERR_FFMPEG_MISSING: &str = "Whisper needs ffmpeg installed on this machine. Install ffmpeg, confirm the `ffmpeg` command works in a terminal, and retry this job.";

pub fn err_download(model: WhisperModel) -> String {
    format!(
        "Could not download Whisper model `{}` ({}) from Hugging Face. Check the network and retry. Incomplete files are not kept.",
        model.as_str(),
        model.size_label()
    )
}

pub fn err_convert(filename: &str) -> String {
    format!(
        "ffmpeg could not convert `{filename}` to 16 kHz WAV. Check that the file plays in a media player and that ffmpeg is up to date, then retry."
    )
}

pub fn err_infer(model: WhisperModel) -> String {
    format!(
        "Whisper transcription (`{}`) failed for this file. Retry the job; if it keeps failing, switch to `tiny` or use AssemblyAI.",
        model.as_str()
    )
}

#[derive(Debug, Clone)]
pub struct WhisperTranscribeOptions {
    pub model_path: PathBuf,
    pub language: Option<String>,
    pub model: WhisperModel,
}

pub trait WhisperEngine: Send + Sync {
    fn transcribe(
        &self,
        wav_path: &Path,
        opts: &WhisperTranscribeOptions,
    ) -> Result<TranscriptionOutcome, String>;
}

pub struct UnimplementedWhisper;

impl WhisperEngine for UnimplementedWhisper {
    fn transcribe(
        &self,
        _: &Path,
        _: &WhisperTranscribeOptions,
    ) -> Result<TranscriptionOutcome, String> {
        Err(err_infer(WhisperModel::Base))
    }
}

#[cfg(test)]
pub struct ScriptedWhisper {
    pub text: String,
    pub fail: bool,
}

#[cfg(test)]
impl WhisperEngine for ScriptedWhisper {
    fn transcribe(
        &self,
        _wav_path: &Path,
        opts: &WhisperTranscribeOptions,
    ) -> Result<TranscriptionOutcome, String> {
        if self.fail {
            return Err("engine boom".into());
        }
        Ok(TranscriptionOutcome {
            text: self.text.clone(),
            metadata: serde_json::json!({
                "model": opts.model.as_str(),
                "language": opts.language,
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_copy_matches_spec() {
        assert!(ERR_FFMPEG_MISSING.contains("ffmpeg"));
        let d = err_download(WhisperModel::Base);
        assert!(d.contains("`base`"));
        assert!(d.contains("~142 MB"));
        assert!(err_convert("clip.ogg").contains("`clip.ogg`"));
        assert!(err_infer(WhisperModel::Tiny).contains("`tiny`"));
    }

    #[test]
    fn scripted_engine_returns_text() {
        let engine = ScriptedWhisper {
            text: "hello from whisper".into(),
            fail: false,
        };
        let out = engine
            .transcribe(
                Path::new("x.wav"),
                &WhisperTranscribeOptions {
                    model_path: PathBuf::from("ggml-base.bin"),
                    language: Some("pt".into()),
                    model: WhisperModel::Base,
                },
            )
            .unwrap();
        assert_eq!(out.text, "hello from whisper");
        assert_eq!(out.metadata["model"], "base");
    }
}
