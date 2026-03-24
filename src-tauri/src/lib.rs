use serde::{Deserialize, Serialize};
use std::{
    env,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tauri::{AppHandle, Manager};

const SESSION_SERVICE_NAME: &str = "com.kimkihong.garmin-run-dashboard.session";
const DATABASE_FILE_NAME: &str = "garmin-run-dashboard.sqlite3";
const LAST_SYNC_SUMMARY_FILE_NAME: &str = "last-sync-summary.json";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LoginSession {
    athlete_name: String,
    issued_at: String,
    expires_at: String,
    token_last_four: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SyncSummary {
    last_synced_at: String,
    raw_activities: u32,
    normalized_activities: u32,
    status: String,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StorageSnapshot {
    app_data_dir: String,
    raw_data_dir: String,
    normalized_data_dir: String,
    database_path: String,
    session_in_keychain: bool,
    last_sync_summary: Option<SyncSummary>,
}

#[derive(Debug)]
struct StoragePaths {
    app_data_dir: PathBuf,
    raw_data_dir: PathBuf,
    normalized_data_dir: PathBuf,
    meta_data_dir: PathBuf,
    database_path: PathBuf,
    last_sync_summary_path: PathBuf,
}

fn current_account_name() -> String {
    env::var("USER").unwrap_or_else(|_| "garmin-run-dashboard-user".to_string())
}

fn resolve_storage_paths(app: &AppHandle) -> Result<StoragePaths, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Unable to resolve app data directory: {error}"))?;

    let raw_data_dir = app_data_dir.join("raw");
    let normalized_data_dir = app_data_dir.join("normalized");
    let meta_data_dir = app_data_dir.join("meta");
    let database_path = app_data_dir.join("db").join(DATABASE_FILE_NAME);
    let last_sync_summary_path = meta_data_dir.join(LAST_SYNC_SUMMARY_FILE_NAME);

    Ok(StoragePaths {
        app_data_dir,
        raw_data_dir,
        normalized_data_dir,
        meta_data_dir,
        database_path,
        last_sync_summary_path,
    })
}

fn ensure_storage_directories(paths: &StoragePaths) -> Result<(), String> {
    let db_dir = paths
        .database_path
        .parent()
        .ok_or_else(|| "Unable to determine the database directory.".to_string())?;

    for directory in [
        &paths.app_data_dir,
        &paths.raw_data_dir,
        &paths.normalized_data_dir,
        &paths.meta_data_dir,
        db_dir,
    ] {
        fs::create_dir_all(directory).map_err(|error| {
            format!(
                "Unable to create the local storage directory {}: {error}",
                directory.display()
            )
        })?;
    }

    Ok(())
}

fn initialize_database(database_path: &Path) -> Result<(), String> {
    let schema = include_str!("../migrations/0001_initial.sql");

    let output = Command::new("sqlite3")
        .arg(database_path)
        .arg(schema)
        .output()
        .map_err(|error| format!("Failed to launch sqlite3 for database bootstrap: {error}"))?;

    if output.status.success() {
        return Ok(());
    }

    Err(format!(
        "Failed to initialize SQLite schema: {}",
        String::from_utf8_lossy(&output.stderr)
    ))
}

fn load_sync_summary_from_disk(paths: &StoragePaths) -> Result<Option<SyncSummary>, String> {
    if !paths.last_sync_summary_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&paths.last_sync_summary_path).map_err(|error| {
        format!(
            "Unable to read sync summary at {}: {error}",
            paths.last_sync_summary_path.display()
        )
    })?;

    serde_json::from_str(&content)
        .map(Some)
        .map_err(|error| format!("Unable to deserialize sync summary JSON: {error}"))
}

fn keychain_session_exists() -> bool {
    let output = Command::new("security")
        .arg("find-generic-password")
        .arg("-a")
        .arg(current_account_name())
        .arg("-s")
        .arg(SESSION_SERVICE_NAME)
        .output();

    matches!(output, Ok(result) if result.status.success())
}

#[cfg(target_os = "macos")]
fn store_session_in_keychain(session: &LoginSession) -> Result<(), String> {
    let payload = serde_json::to_string(session)
        .map_err(|error| format!("Unable to serialize the login session: {error}"))?;

    let output = Command::new("security")
        .arg("add-generic-password")
        .arg("-U")
        .arg("-a")
        .arg(current_account_name())
        .arg("-s")
        .arg(SESSION_SERVICE_NAME)
        .arg("-w")
        .arg(payload)
        .output()
        .map_err(|error| format!("Unable to launch the macOS security tool: {error}"))?;

    if output.status.success() {
        return Ok(());
    }

    Err(format!(
        "Unable to store the session in the macOS Keychain: {}",
        String::from_utf8_lossy(&output.stderr)
    ))
}

#[cfg(not(target_os = "macos"))]
fn store_session_in_keychain(_session: &LoginSession) -> Result<(), String> {
    Err("Secure session storage is currently implemented for macOS only.".to_string())
}

#[cfg(target_os = "macos")]
fn load_session_from_keychain() -> Result<Option<LoginSession>, String> {
    let output = Command::new("security")
        .arg("find-generic-password")
        .arg("-a")
        .arg(current_account_name())
        .arg("-s")
        .arg(SESSION_SERVICE_NAME)
        .arg("-w")
        .output()
        .map_err(|error| format!("Unable to query the macOS Keychain: {error}"))?;

    if !output.status.success() {
        return Ok(None);
    }

    let payload = String::from_utf8(output.stdout)
        .map_err(|error| format!("Keychain payload was not valid UTF-8: {error}"))?;

    serde_json::from_str::<LoginSession>(payload.trim())
        .map(Some)
        .map_err(|error| format!("Unable to deserialize the stored login session: {error}"))
}

#[cfg(not(target_os = "macos"))]
fn load_session_from_keychain() -> Result<Option<LoginSession>, String> {
    Ok(None)
}

#[cfg(target_os = "macos")]
fn clear_session_from_keychain() -> Result<(), String> {
    let output = Command::new("security")
        .arg("delete-generic-password")
        .arg("-a")
        .arg(current_account_name())
        .arg("-s")
        .arg(SESSION_SERVICE_NAME)
        .output()
        .map_err(|error| format!("Unable to launch the macOS security tool: {error}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);

    if stderr.contains("could not be found") {
        return Ok(());
    }

    Err(format!(
        "Unable to remove the stored session from the macOS Keychain: {stderr}"
    ))
}

#[cfg(not(target_os = "macos"))]
fn clear_session_from_keychain() -> Result<(), String> {
    Ok(())
}

fn prepare_local_store(app: &AppHandle) -> Result<StorageSnapshot, String> {
    let paths = resolve_storage_paths(app)?;
    ensure_storage_directories(&paths)?;
    initialize_database(&paths.database_path)?;
    let last_sync_summary = load_sync_summary_from_disk(&paths)?;

    Ok(StorageSnapshot {
        app_data_dir: paths.app_data_dir.display().to_string(),
        raw_data_dir: paths.raw_data_dir.display().to_string(),
        normalized_data_dir: paths.normalized_data_dir.display().to_string(),
        database_path: paths.database_path.display().to_string(),
        session_in_keychain: keychain_session_exists(),
        last_sync_summary,
    })
}

#[tauri::command]
fn bootstrap_local_store(app: AppHandle) -> Result<StorageSnapshot, String> {
    prepare_local_store(&app)
}

#[tauri::command]
fn store_login_session(session: LoginSession) -> Result<(), String> {
    store_session_in_keychain(&session)
}

#[tauri::command]
fn load_login_session() -> Result<Option<LoginSession>, String> {
    load_session_from_keychain()
}

#[tauri::command]
fn clear_login_session() -> Result<(), String> {
    clear_session_from_keychain()
}

#[tauri::command]
fn save_sync_summary(app: AppHandle, summary: SyncSummary) -> Result<(), String> {
    let paths = resolve_storage_paths(&app)?;
    ensure_storage_directories(&paths)?;

    let payload = serde_json::to_string_pretty(&summary)
        .map_err(|error| format!("Unable to serialize sync summary: {error}"))?;

    fs::write(&paths.last_sync_summary_path, payload).map_err(|error| {
        format!(
            "Unable to write sync summary to {}: {error}",
            paths.last_sync_summary_path.display()
        )
    })
}

#[tauri::command]
fn load_sync_summary(app: AppHandle) -> Result<Option<SyncSummary>, String> {
    let paths = resolve_storage_paths(&app)?;
    load_sync_summary_from_disk(&paths)
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            prepare_local_store(&app.handle())
                .map(|_| ())
                .map_err(|error| -> Box<dyn std::error::Error> { error.into() })?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap_local_store,
            store_login_session,
            load_login_session,
            clear_login_session,
            save_sync_summary,
            load_sync_summary,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
