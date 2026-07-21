//! Workspace settings persisted in SQLite (no tenant/user).

use crate::domain::models::{AppSettings, UpdateSettingsInput};
use rusqlite::Connection;

const ALLOWED_PROVIDERS: &[&str] = &["assemblyai", "whisper"];
const ALLOWED_LANGUAGES: &[&str] = &["auto", "pt", "en", "es"];

pub fn get_settings(conn: &Connection) -> Result<AppSettings, String> {
    conn.query_row(
        "SELECT workspace_name, default_provider, assemblyai_api_key, language
         FROM app_settings WHERE id = 1",
        [],
        |row| {
            let key: Option<String> = row.get(2)?;
            let has = key.as_ref().map(|k| !k.trim().is_empty()).unwrap_or(false);
            Ok(AppSettings {
                workspace_name: row.get(0)?,
                default_provider: row.get(1)?,
                assemblyai_api_key: if has { key } else { None },
                has_api_key: has,
                language: row.get(3)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

pub fn update_settings(
    conn: &Connection,
    input: UpdateSettingsInput,
) -> Result<AppSettings, String> {
    let workspace_name = input.workspace_name.trim().to_string();
    if workspace_name.is_empty() {
        return Err("Workspace name is required".into());
    }

    let default_provider = input.default_provider.trim().to_lowercase();
    if !ALLOWED_PROVIDERS.contains(&default_provider.as_str()) {
        return Err(format!("Provider '{default_provider}' is not allowed"));
    }

    let language = input.language.trim().to_lowercase();
    if !ALLOWED_LANGUAGES.contains(&language.as_str()) {
        return Err(format!("Language '{language}' is not allowed"));
    }

    let existing = get_settings(conn)?;
    let api_key = match input.assemblyai_api_key {
        Some(k) if !k.trim().is_empty() => Some(k.trim().to_string()),
        Some(_) => None, // explicit clear
        None => existing.assemblyai_api_key.clone(),
    };

    if default_provider == "assemblyai" && api_key.as_ref().map(|k| k.is_empty()).unwrap_or(true)
    {
        // allow saving workspace without key, but mark provider needs key at run time
    }

    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE app_settings SET
            workspace_name = ?1,
            default_provider = ?2,
            assemblyai_api_key = ?3,
            language = ?4,
            updated_at = ?5
         WHERE id = 1",
        rusqlite::params![
            workspace_name,
            default_provider,
            api_key,
            language,
            now
        ],
    )
    .map_err(|e| e.to_string())?;

    get_settings(conn)
}

pub fn assemblyai_api_key(conn: &Connection) -> Result<Option<String>, String> {
    let settings = get_settings(conn)?;
    Ok(settings.assemblyai_api_key.filter(|k| !k.trim().is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use tempfile::tempdir;

    #[test]
    fn rejects_empty_workspace_and_invalid_provider() {
        let dir = tempdir().unwrap();
        let conn = db::open(&dir.path().join("s.db")).unwrap();

        let err = update_settings(
            &conn,
            UpdateSettingsInput {
                workspace_name: "   ".into(),
                default_provider: "assemblyai".into(),
                assemblyai_api_key: None,
                language: "auto".into(),
            },
        )
        .unwrap_err();
        assert!(err.to_lowercase().contains("workspace"));

        let err = update_settings(
            &conn,
            UpdateSettingsInput {
                workspace_name: "Ok".into(),
                default_provider: "bogus".into(),
                assemblyai_api_key: None,
                language: "auto".into(),
            },
        )
        .unwrap_err();
        assert!(err.to_lowercase().contains("not allowed") || err.contains("bogus"));
    }

    #[test]
    fn omit_api_key_keeps_existing() {
        let dir = tempdir().unwrap();
        let conn = db::open(&dir.path().join("s.db")).unwrap();
        update_settings(
            &conn,
            UpdateSettingsInput {
                workspace_name: "A".into(),
                default_provider: "assemblyai".into(),
                assemblyai_api_key: Some("fixture-key-not-real".into()),
                language: "en".into(),
            },
        )
        .unwrap();

        let updated = update_settings(
            &conn,
            UpdateSettingsInput {
                workspace_name: "B".into(),
                default_provider: "assemblyai".into(),
                assemblyai_api_key: None, // omit → keep
                language: "pt".into(),
            },
        )
        .unwrap();
        assert_eq!(updated.workspace_name, "B");
        assert_eq!(
            updated.assemblyai_api_key.as_deref(),
            Some("fixture-key-not-real")
        );
        assert_eq!(updated.language, "pt");
    }

    #[test]
    fn settings_roundtrip() {
        let dir = tempdir().unwrap();
        let conn = db::open(&dir.path().join("s.db")).unwrap();

        let initial = get_settings(&conn).unwrap();
        assert_eq!(initial.workspace_name, "Local");
        assert_eq!(initial.default_provider, "assemblyai");
        assert!(!initial.has_api_key);

        let updated = update_settings(
            &conn,
            UpdateSettingsInput {
                workspace_name: "Studio".into(),
                default_provider: "assemblyai".into(),
                assemblyai_api_key: Some("fixture-key-not-real".into()),
                language: "pt".into(),
            },
        )
        .unwrap();

        assert_eq!(updated.workspace_name, "Studio");
        assert_eq!(
            updated.assemblyai_api_key.as_deref(),
            Some("fixture-key-not-real")
        );
        assert!(updated.has_api_key);
        assert_eq!(updated.language, "pt");

        let reloaded = get_settings(&conn).unwrap();
        assert_eq!(reloaded.workspace_name, "Studio");
        assert_eq!(
            reloaded.assemblyai_api_key.as_deref(),
            Some("fixture-key-not-real")
        );
    }
}
