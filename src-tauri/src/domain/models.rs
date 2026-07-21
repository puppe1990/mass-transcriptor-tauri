use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub workspace_name: String,
    pub default_provider: String,
    pub assemblyai_api_key: Option<String>,
    pub has_api_key: bool,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsInput {
    pub workspace_name: String,
    pub default_provider: String,
    /// When omitted (`None`), keep the existing key. When `Some("")`, clear it.
    #[serde(default)]
    pub assemblyai_api_key: Option<String>,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSummary {
    pub id: i64,
    pub status: String,
    pub provider_key: String,
    pub batch_id: Option<i64>,
    pub upload_id: i64,
    pub original_filename: String,
    pub error_message: Option<String>,
    pub created_at: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobDetail {
    pub id: i64,
    pub status: String,
    pub provider_key: String,
    pub batch_id: Option<i64>,
    pub upload_id: i64,
    pub original_filename: String,
    pub error_message: Option<String>,
    pub markdown_path: Option<String>,
    pub transcript_text: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub retryable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchDetail {
    pub id: i64,
    pub created_at: String,
    pub jobs: Vec<JobDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedJob {
    pub id: i64,
    pub upload_id: i64,
    pub batch_id: Option<i64>,
    pub status: String,
    pub original_filename: String,
}

#[derive(Debug, Clone)]
pub struct NewUploadFile {
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub source_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionOutcome {
    pub text: String,
    pub metadata: serde_json::Value,
}
