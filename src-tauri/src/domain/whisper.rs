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

pub fn err_not_downloaded(model: WhisperModel) -> String {
    format!(
        "Whisper model `{}` ({}) is not downloaded. Open Settings, click Download for that model, then retry this job. Models are not downloaded automatically.",
        model.as_str(),
        model.size_label()
    )
}

pub fn err_corrupt(model: WhisperModel) -> String {
    format!(
        "Whisper model `{}` on disk is corrupted. Delete it in Settings and download it again.",
        model.as_str()
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

pub struct WhisperRsEngine;

impl WhisperEngine for WhisperRsEngine {
    fn transcribe(
        &self,
        wav_path: &Path,
        opts: &WhisperTranscribeOptions,
    ) -> Result<TranscriptionOutcome, String> {
        let model_path = opts
            .model_path
            .to_str()
            .ok_or_else(|| "invalid model path".to_string())?;
        let ctx = whisper_rs::WhisperContext::new_with_params(
            model_path,
            whisper_rs::WhisperContextParameters::default(),
        )
        .map_err(|e| e.to_string())?;
        let mut state = ctx.create_state().map_err(|e| e.to_string())?;

        let mut reader = hound::WavReader::open(wav_path).map_err(|e| e.to_string())?;
        let samples_i16: Vec<i16> = reader
            .samples::<i16>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let mut audio = vec![0.0f32; samples_i16.len()];
        whisper_rs::convert_integer_to_float_audio(&samples_i16, &mut audio)
            .map_err(|e| e.to_string())?;

        let mut params =
            whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
        params.set_translate(false);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        match opts.language.as_deref() {
            Some(lang) => params.set_language(Some(lang)),
            None => params.set_language(None),
        }

        state.full(params, &audio).map_err(|e| e.to_string())?;
        // whisper-rs 0.16: full_n_segments returns c_int; text is on WhisperSegment via as_iter().
        let mut text = String::new();
        for segment in state.as_iter() {
            text.push_str(segment.to_str().map_err(|e| e.to_string())?);
        }
        Ok(TranscriptionOutcome {
            text: text.trim().to_string(),
            metadata: serde_json::json!({
                "model": opts.model.as_str(),
                "language": opts.language,
            }),
        })
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
        let missing = err_not_downloaded(WhisperModel::Small);
        assert!(missing.contains("`small`"));
        assert!(missing.contains("not downloaded automatically"));
        assert!(err_corrupt(WhisperModel::MediumQ8).contains("`medium-q8`"));
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
