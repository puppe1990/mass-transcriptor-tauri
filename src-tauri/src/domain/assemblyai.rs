//! AssemblyAI client with injectable HTTP transport for tests.

use crate::domain::models::TranscriptionOutcome;
use serde_json::{json, Value};
use std::path::Path;
use std::thread;
use std::time::Duration;

pub const BASE_URL: &str = "https://api.assemblyai.com/v2";
const SPEECH_MODELS: &[&str] = &["universal-3-pro", "universal-2"];

/// Injectable HTTP layer so domain tests drive the real client without the network.
pub trait HttpTransport: Send + Sync {
    fn post_bytes(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: Vec<u8>,
    ) -> Result<Value, String>;

    fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &Value,
    ) -> Result<Value, String>;

    fn get_json(&self, url: &str, headers: &[(&str, &str)]) -> Result<Value, String>;
}

/// Production transport using reqwest blocking client.
pub struct ReqwestTransport {
    client: reqwest::blocking::Client,
}

impl Default for ReqwestTransport {
    fn default() -> Self {
        Self {
            client: reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .expect("reqwest client"),
        }
    }
}

impl HttpTransport for ReqwestTransport {
    fn post_bytes(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: Vec<u8>,
    ) -> Result<Value, String> {
        let mut req = self.client.post(url).body(body);
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        let resp = req.send().map_err(|e| e.to_string())?;
        let status = resp.status();
        let text = resp.text().map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("HTTP {status}: {text}"));
        }
        serde_json::from_str(&text).map_err(|e| e.to_string())
    }

    fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &Value,
    ) -> Result<Value, String> {
        let mut req = self.client.post(url).json(body);
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        let resp = req.send().map_err(|e| e.to_string())?;
        let status = resp.status();
        let text = resp.text().map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("HTTP {status}: {text}"));
        }
        serde_json::from_str(&text).map_err(|e| e.to_string())
    }

    fn get_json(&self, url: &str, headers: &[(&str, &str)]) -> Result<Value, String> {
        let mut req = self.client.get(url);
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        let resp = req.send().map_err(|e| e.to_string())?;
        let status = resp.status();
        let text = resp.text().map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("HTTP {status}: {text}"));
        }
        serde_json::from_str(&text).map_err(|e| e.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct TranscribeOptions {
    pub api_key: String,
    pub language: Option<String>,
    pub poll_interval: Duration,
    pub max_polls: u32,
}

impl Default for TranscribeOptions {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            language: None,
            poll_interval: Duration::from_secs(3),
            max_polls: 60,
        }
    }
}

/// Upload → create transcript → poll until completed/error.
pub fn transcribe(
    transport: &dyn HttpTransport,
    file_path: &Path,
    opts: &TranscribeOptions,
) -> Result<TranscriptionOutcome, String> {
    let bytes = std::fs::read(file_path).map_err(|e| e.to_string())?;
    let auth = [("authorization", opts.api_key.as_str())];

    let upload_body = transport.post_bytes(
        &format!("{BASE_URL}/upload"),
        &auth,
        bytes,
    )?;
    let upload_url = upload_body
        .get("upload_url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "AssemblyAI upload failed".to_string())?
        .to_string();

    let mut body = json!({
        "audio_url": upload_url,
        "speech_models": SPEECH_MODELS,
    });
    match opts.language.as_deref() {
        None | Some("auto") => {
            body["language_detection"] = json!(true);
        }
        Some(lang) => {
            body["language_code"] = json!(lang);
        }
    }

    let create_headers = [
        ("authorization", opts.api_key.as_str()),
        ("content-type", "application/json"),
    ];
    let created = transport.post_json(
        &format!("{BASE_URL}/transcript"),
        &create_headers,
        &body,
    )?;
    let transcript_id = created
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "AssemblyAI transcript request failed".to_string())?
        .to_string();

    for attempt in 1..=opts.max_polls {
        let status_body = transport.get_json(
            &format!("{BASE_URL}/transcript/{transcript_id}"),
            &auth,
        )?;
        let status = status_body
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        match status {
            "completed" => {
                let text = status_body
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let mut metadata = json!({ "id": transcript_id });
                if let Some(lang) = status_body.get("language_code") {
                    metadata["language_code"] = lang.clone();
                }
                return Ok(TranscriptionOutcome { text, metadata });
            }
            "error" => {
                let msg = status_body
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("AssemblyAI transcription failed");
                return Err(msg.to_string());
            }
            "queued" | "processing" => {
                if attempt < opts.max_polls && !opts.poll_interval.is_zero() {
                    thread::sleep(opts.poll_interval);
                }
            }
            _ => return Err("AssemblyAI transcription failed".into()),
        }
    }

    Err("AssemblyAI transcription timed out".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use std::sync::Arc;
    use tempfile::NamedTempFile;

    struct ScriptedTransport {
        calls: Mutex<Vec<String>>,
        responses: Mutex<Vec<Result<Value, String>>>,
    }

    impl ScriptedTransport {
        fn new(responses: Vec<Result<Value, String>>) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                responses: Mutex::new(responses),
            }
        }
    }

    impl HttpTransport for ScriptedTransport {
        fn post_bytes(
            &self,
            url: &str,
            _headers: &[(&str, &str)],
            _body: Vec<u8>,
        ) -> Result<Value, String> {
            self.calls.lock().push(format!("POST_BYTES {url}"));
            self.responses.lock().remove(0)
        }

        fn post_json(
            &self,
            url: &str,
            _headers: &[(&str, &str)],
            _body: &Value,
        ) -> Result<Value, String> {
            self.calls.lock().push(format!("POST_JSON {url}"));
            self.responses.lock().remove(0)
        }

        fn get_json(&self, url: &str, _headers: &[(&str, &str)]) -> Result<Value, String> {
            self.calls.lock().push(format!("GET {url}"));
            self.responses.lock().remove(0)
        }
    }

    #[test]
    fn success_path_writes_text_and_metadata() {
        let file = NamedTempFile::new().unwrap();
        std::fs::write(file.path(), b"fake-audio").unwrap();

        let transport = ScriptedTransport::new(vec![
            Ok(json!({"upload_url": "https://cdn.example/a.wav"})),
            Ok(json!({"id": "tr_123"})),
            Ok(json!({"status": "processing"})),
            Ok(json!({
                "status": "completed",
                "text": "hello from assembly",
                "language_code": "en",
                "id": "tr_123"
            })),
        ]);

        let outcome = transcribe(
            &transport,
            file.path(),
            &TranscribeOptions {
                api_key: "key".into(),
                language: Some("en".into()),
                poll_interval: Duration::ZERO,
                max_polls: 5,
            },
        )
        .unwrap();

        assert_eq!(outcome.text, "hello from assembly");
        assert_eq!(outcome.metadata["id"], "tr_123");
        assert_eq!(outcome.metadata["language_code"], "en");
        let calls = transport.calls.lock().clone();
        assert!(calls[0].contains("/upload"));
        assert!(calls[1].contains("/transcript"));
    }

    #[test]
    fn failure_path_returns_provider_error() {
        let file = NamedTempFile::new().unwrap();
        std::fs::write(file.path(), b"fake-audio").unwrap();

        let transport = ScriptedTransport::new(vec![
            Ok(json!({"upload_url": "https://cdn.example/a.wav"})),
            Ok(json!({"id": "tr_err"})),
            Ok(json!({"status": "error", "error": "bad audio"})),
        ]);

        let err = transcribe(
            &transport,
            file.path(),
            &TranscribeOptions {
                api_key: "key".into(),
                language: None,
                poll_interval: Duration::ZERO,
                max_polls: 3,
            },
        )
        .unwrap_err();

        assert_eq!(err, "bad audio");
    }

    // silence unused import warning for Arc in case we extend later
    #[allow(dead_code)]
    fn _arc_hint() {
        let _: Arc<()> = Arc::new(());
    }
}
