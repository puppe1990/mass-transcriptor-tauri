//! Tauri desktop shell for Mass Transcriptor — no login, local SQLite.

mod app_state;
mod db;
mod domain;

use app_state::AppState;
use domain::assemblyai::ReqwestTransport;
use domain::grouping::JobListRow;
use domain::models::{
    AppSettings, BatchDetail, CreatedJob, JobDetail, JobSummary, NewUploadFile, UpdateSettingsInput,
};
use domain::{jobs, settings};
use std::path::PathBuf;
use tauri::image::Image;
use tauri::{AppHandle, Emitter, Manager, State};

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let conn = state.db.lock();
    settings::get_settings(&conn)
}

#[tauri::command]
fn update_settings(
    state: State<'_, AppState>,
    input: UpdateSettingsInput,
) -> Result<AppSettings, String> {
    let conn = state.db.lock();
    settings::update_settings(&conn, input)
}

#[tauri::command]
fn create_jobs_from_paths(
    state: State<'_, AppState>,
    app: AppHandle,
    paths: Vec<String>,
) -> Result<Vec<CreatedJob>, String> {
    if paths.is_empty() {
        return Err("No files selected".into());
    }

    let files: Vec<NewUploadFile> = paths
        .into_iter()
        .map(|source_path| {
            let path = PathBuf::from(&source_path);
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("audio")
                .to_string();
            let size_bytes = std::fs::metadata(&path).map(|m| m.len() as i64).unwrap_or(0);
            let mime_type = mime_guess::from_path(&path)
                .first_or_octet_stream()
                .essence_str()
                .to_string();
            NewUploadFile {
                filename,
                mime_type,
                size_bytes,
                source_path,
            }
        })
        .collect();

    let created = {
        let conn = state.db.lock();
        let storage = state.storage();
        jobs::create_uploads_and_jobs(&conn, &storage, &files)?
    };

    // Kick off background processing for each job
    for job in &created {
        spawn_process_job(app.clone(), state.inner().clone(), job.id);
    }

    Ok(created)
}

/// Write a browser/HTML5-dropped file into a temp path so the job pipeline can read it.
#[tauri::command]
fn write_temp_upload(filename: String, bytes: Vec<u8>) -> Result<String, String> {
    let safe = PathBuf::from(&filename)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("upload.bin")
        .to_string();
    let dir = std::env::temp_dir().join("mass-transcriptor-uploads");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stamp = chrono::Utc::now().timestamp_millis();
    let path = dir.join(format!("{stamp}_{safe}"));
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

fn spawn_process_job(app: AppHandle, state: AppState, job_id: i64) {
    std::thread::spawn(move || {
        let transport = ReqwestTransport::default();
        let storage = state.storage();
        // Uses short DB locks only around prepare/finish — not during AssemblyAI network I/O.
        let result =
            jobs::process_transcription_job_with_lock(&state.db, &storage, &transport, job_id);
        let _ = app.emit(
            "job-updated",
            serde_json::json!({
                "jobId": job_id,
                "ok": result.is_ok(),
                "error": result.err(),
            }),
        );
    });
}

#[tauri::command]
fn list_jobs(state: State<'_, AppState>) -> Result<Vec<JobSummary>, String> {
    let conn = state.db.lock();
    jobs::list_jobs(&conn)
}

#[tauri::command]
fn list_job_rows(state: State<'_, AppState>) -> Result<Vec<JobListRow>, String> {
    let conn = state.db.lock();
    jobs::list_job_rows(&conn)
}

#[tauri::command]
fn get_job(state: State<'_, AppState>, job_id: i64) -> Result<Option<JobDetail>, String> {
    let conn = state.db.lock();
    jobs::get_job_detail(&conn, job_id)
}

#[tauri::command]
fn get_batch(state: State<'_, AppState>, batch_id: i64) -> Result<Option<BatchDetail>, String> {
    let conn = state.db.lock();
    jobs::get_batch_detail(&conn, batch_id)
}

#[tauri::command]
fn retry_job(
    state: State<'_, AppState>,
    app: AppHandle,
    job_id: i64,
) -> Result<JobDetail, String> {
    let detail = {
        let conn = state.db.lock();
        jobs::retry_job(&conn, job_id)?
    };
    spawn_process_job(app, state.inner().clone(), job_id);
    Ok(detail)
}

#[tauri::command]
fn get_transcript_markdown(state: State<'_, AppState>, job_id: i64) -> Result<String, String> {
    let conn = state.db.lock();
    jobs::read_transcript_markdown(&conn, job_id)
}

#[tauri::command]
fn app_info(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "dbPath": state.db_path.to_string_lossy(),
        "storageRoot": state.storage_root.to_string_lossy(),
        "auth": false,
        "entry": "main",
    }))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| e.to_string())?;
            let state = AppState::initialize(data_dir)?;
            app.manage(state);

            // Force window/dock icon in dev + release (bundle icon alone can lag in `tauri dev`).
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(icon) = Image::from_bytes(include_bytes!("../icons/icon.png")) {
                    let _ = window.set_icon(icon);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            update_settings,
            create_jobs_from_paths,
            write_temp_upload,
            list_jobs,
            list_job_rows,
            get_job,
            get_batch,
            retry_job,
            get_transcript_markdown,
            app_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
