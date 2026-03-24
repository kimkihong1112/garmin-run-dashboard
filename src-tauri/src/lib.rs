use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    env,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Mutex,
};
use tauri::{AppHandle, Manager, State};

const SESSION_SERVICE_NAME: &str = "com.kimkihong.garmin-run-dashboard.session";
const DATABASE_FILE_NAME: &str = "garmin-run-dashboard.sqlite3";
const LAST_SYNC_SUMMARY_FILE_NAME: &str = "last-sync-summary.json";
const DEFAULT_SYNC_ACTIVITY_LIMIT: usize = 120;

#[derive(Default)]
struct GarminRuntimeState {
    pending_mfa_challenge: Mutex<Option<PendingMfaChallenge>>,
}

#[derive(Debug, Clone)]
struct PendingMfaChallenge {
    account_email: String,
    challenge_state: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginCredentials {
    email: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LoginSession {
    athlete_name: String,
    full_name: Option<String>,
    account_email: String,
    issued_at: String,
    expires_at: String,
    token_last_four: String,
    unit_system: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct StoredSession {
    session: LoginSession,
    token_store: String,
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
    garmin_adapter_ready: bool,
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

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum GarminAuthResponse {
    MfaRequired { message: String },
    Authenticated { session: LoginSession },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum GarminAdapterAuthResponse {
    MfaRequired {
        message: String,
        challenge_state: String,
        account_email: String,
    },
    Authenticated {
        session: LoginSession,
        token_store: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GarminSyncActivity {
    activity_id: String,
    activity_name: String,
    activity_type: String,
    source_created_at: String,
    start_time_local: String,
    start_time_gmt: String,
    time_zone: String,
    distance_meters: Option<f64>,
    duration_seconds: Option<f64>,
    average_pace_seconds_per_km: Option<f64>,
    average_heart_rate: Option<f64>,
    max_heart_rate: Option<f64>,
    elevation_gain_meters: Option<f64>,
    training_load: Option<f64>,
    raw_file_path: String,
    summary_json: Value,
    normalized_json: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GarminAdapterSyncResponse {
    session: LoginSession,
    token_store: String,
    activities: Vec<GarminSyncActivity>,
    warnings: Vec<String>,
}

fn current_account_name() -> String {
    env::var("USER").unwrap_or_else(|_| "garmin-run-dashboard-user".to_string())
}

fn now_iso_string() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn workspace_root() -> Result<PathBuf, String> {
    env::current_dir().map_err(|error| format!("Unable to resolve the workspace root: {error}"))
}

fn resolve_adapter_script_path() -> Result<PathBuf, String> {
    let script_path = workspace_root()?.join("scripts").join("garmin_adapter.py");

    if script_path.exists() {
        Ok(script_path)
    } else {
        Err(format!(
            "The Garmin adapter script was not found at {}.",
            script_path.display()
        ))
    }
}

fn resolve_python_executable() -> Result<PathBuf, String> {
    let workspace = workspace_root()?;
    let candidates = [
        workspace.join(".venv-garmin").join("bin").join("python"),
        workspace.join(".venv-garmin").join("bin").join("python3.12"),
        PathBuf::from("/opt/homebrew/bin/python3.12"),
        PathBuf::from("/usr/local/bin/python3.12"),
        PathBuf::from("python3.12"),
        PathBuf::from("python3"),
    ];

    candidates
        .into_iter()
        .find(|path| path.exists() || path.is_relative())
        .ok_or_else(|| {
            "Python 3.12 or the local Garmin adapter virtualenv could not be found.".to_string()
        })
}

fn garmin_adapter_ready() -> bool {
    resolve_adapter_script_path().is_ok() && resolve_python_executable().is_ok()
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

fn write_sync_summary_to_disk(paths: &StoragePaths, summary: &SyncSummary) -> Result<(), String> {
    let payload = serde_json::to_string_pretty(summary)
        .map_err(|error| format!("Unable to serialize sync summary: {error}"))?;

    fs::write(&paths.last_sync_summary_path, payload).map_err(|error| {
        format!(
            "Unable to write sync summary to {}: {error}",
            paths.last_sync_summary_path.display()
        )
    })
}

fn clear_sync_summary_from_disk(paths: &StoragePaths) -> Result<(), String> {
    if paths.last_sync_summary_path.exists() {
        fs::remove_file(&paths.last_sync_summary_path).map_err(|error| {
            format!(
                "Unable to remove sync summary at {}: {error}",
                paths.last_sync_summary_path.display()
            )
        })?;
    }

    Ok(())
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
fn store_session_in_keychain(session: &StoredSession) -> Result<(), String> {
    let payload = serde_json::to_string(session)
        .map_err(|error| format!("Unable to serialize the secure login session: {error}"))?;

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
fn store_session_in_keychain(_session: &StoredSession) -> Result<(), String> {
    Err("Secure session storage is currently implemented for macOS only.".to_string())
}

#[cfg(target_os = "macos")]
fn load_session_from_keychain() -> Result<Option<StoredSession>, String> {
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

    serde_json::from_str::<StoredSession>(payload.trim())
        .map(Some)
        .map_err(|error| format!("Unable to deserialize the stored login session: {error}"))
}

#[cfg(not(target_os = "macos"))]
fn load_session_from_keychain() -> Result<Option<StoredSession>, String> {
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
        garmin_adapter_ready: garmin_adapter_ready(),
        session_in_keychain: keychain_session_exists(),
        last_sync_summary,
    })
}

fn run_python_adapter(command_name: &str, payload: &Value) -> Result<String, String> {
    let python = resolve_python_executable()?;
    let script = resolve_adapter_script_path()?;

    let mut child = Command::new(&python)
        .arg(&script)
        .arg(command_name)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            format!(
                "Unable to launch the Garmin adapter with {} {}: {error}",
                python.display(),
                script.display()
            )
        })?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(payload.to_string().as_bytes())
            .map_err(|error| format!("Unable to send input to the Garmin adapter: {error}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|error| format!("Unable to read the Garmin adapter output: {error}"))?;

    if output.status.success() {
        return String::from_utf8(output.stdout)
            .map_err(|error| format!("Garmin adapter stdout was not valid UTF-8: {error}"));
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if !stderr.is_empty() { stderr } else { stdout };

    Err(if detail.is_empty() {
        "The Garmin adapter exited without a useful error message.".to_string()
    } else {
        detail
    })
}

fn sql_text(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn sql_number(value: f64) -> String {
    if value.is_finite() {
        value.to_string()
    } else {
        "0".to_string()
    }
}

fn sql_nullable_number(value: Option<f64>) -> String {
    value
        .filter(|number| number.is_finite())
        .map(|number| number.to_string())
        .unwrap_or_else(|| "NULL".to_string())
}

fn execute_sql(database_path: &Path, sql: &str) -> Result<String, String> {
    let output = Command::new("sqlite3")
        .arg(database_path)
        .arg(sql)
        .output()
        .map_err(|error| format!("Failed to launch sqlite3: {error}"))?;

    if output.status.success() {
        return String::from_utf8(output.stdout)
            .map_err(|error| format!("sqlite3 output was not valid UTF-8: {error}"));
    }

    Err(format!(
        "sqlite3 execution failed: {}",
        String::from_utf8_lossy(&output.stderr)
    ))
}

fn query_count(database_path: &Path, table_name: &str) -> Result<u32, String> {
    let output = Command::new("sqlite3")
        .arg("-json")
        .arg(database_path)
        .arg(format!("SELECT COUNT(*) AS count FROM {table_name};"))
        .output()
        .map_err(|error| format!("Failed to query sqlite3: {error}"))?;

    if !output.status.success() {
        return Err(format!(
            "sqlite3 count query failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("sqlite3 count output was not valid UTF-8: {error}"))?;

    let rows: Vec<Value> = serde_json::from_str(stdout.trim())
        .map_err(|error| format!("Unable to parse sqlite3 count output: {error}"))?;

    let count = rows
        .first()
        .and_then(|row| row.get("count"))
        .and_then(Value::as_u64)
        .unwrap_or(0);

    Ok(count as u32)
}

fn persist_synced_activities(
    database_path: &Path,
    session: &LoginSession,
    activities: &[GarminSyncActivity],
) -> Result<(), String> {
    let athlete_id = session.account_email.trim().to_lowercase();
    let timezone = activities
        .first()
        .map(|activity| activity.time_zone.as_str())
        .unwrap_or("UTC");

    let mut statements = Vec::with_capacity(activities.len() * 2 + 3);
    statements.push("BEGIN IMMEDIATE;".to_string());
    statements.push(format!(
        "INSERT INTO athletes (id, garmin_athlete_id, display_name, timezone) \
         VALUES ({}, NULL, {}, {}) \
         ON CONFLICT(id) DO UPDATE SET display_name = excluded.display_name, timezone = excluded.timezone;",
        sql_text(&athlete_id),
        sql_text(&session.athlete_name),
        sql_text(timezone),
    ));

    for activity in activities {
        let summary_payload = serde_json::to_string(&activity.summary_json)
            .map_err(|error| format!("Unable to serialize Garmin summary JSON: {error}"))?;
        let normalized_payload = serde_json::to_string(&json!({
            "activityName": activity.activity_name,
            "activityType": activity.activity_type,
            "rawFilePath": activity.raw_file_path,
            "normalized": activity.normalized_json,
        }))
        .map_err(|error| format!("Unable to serialize normalized Garmin JSON: {error}"))?;

        statements.push(format!(
            "INSERT INTO activities_raw (id, athlete_id, garmin_activity_id, activity_type, source_created_at, payload_json) \
             VALUES ({id}, {athlete_id}, {activity_id}, {activity_type}, {created_at}, {payload_json}) \
             ON CONFLICT(garmin_activity_id) DO UPDATE SET \
             athlete_id = excluded.athlete_id, \
             activity_type = excluded.activity_type, \
             source_created_at = excluded.source_created_at, \
             payload_json = excluded.payload_json;",
            id = sql_text(&activity.activity_id),
            athlete_id = sql_text(&athlete_id),
            activity_id = sql_text(&activity.activity_id),
            activity_type = sql_text(&activity.activity_type),
            created_at = sql_text(&activity.source_created_at),
            payload_json = sql_text(&summary_payload),
        ));

        statements.push(format!(
            "INSERT INTO activities_normalized (activity_id, athlete_id, local_start_at, utc_start_at, distance_meters, duration_seconds, average_pace_seconds_per_km, average_heart_rate, max_heart_rate, elevation_gain_meters, training_load, normalized_json) \
             VALUES ({activity_id}, {athlete_id}, {local_start}, {utc_start}, {distance}, {duration}, {pace}, {average_hr}, {max_hr}, {elevation}, {training_load}, {normalized_json}) \
             ON CONFLICT(activity_id) DO UPDATE SET \
             athlete_id = excluded.athlete_id, \
             local_start_at = excluded.local_start_at, \
             utc_start_at = excluded.utc_start_at, \
             distance_meters = excluded.distance_meters, \
             duration_seconds = excluded.duration_seconds, \
             average_pace_seconds_per_km = excluded.average_pace_seconds_per_km, \
             average_heart_rate = excluded.average_heart_rate, \
             max_heart_rate = excluded.max_heart_rate, \
             elevation_gain_meters = excluded.elevation_gain_meters, \
             training_load = excluded.training_load, \
             normalized_json = excluded.normalized_json;",
            activity_id = sql_text(&activity.activity_id),
            athlete_id = sql_text(&athlete_id),
            local_start = sql_text(&activity.start_time_local),
            utc_start = sql_text(&activity.start_time_gmt),
            distance = sql_number(activity.distance_meters.unwrap_or(0.0)),
            duration = sql_number(activity.duration_seconds.unwrap_or(0.0)),
            pace = sql_nullable_number(activity.average_pace_seconds_per_km),
            average_hr = sql_nullable_number(activity.average_heart_rate),
            max_hr = sql_nullable_number(activity.max_heart_rate),
            elevation = sql_nullable_number(activity.elevation_gain_meters),
            training_load = sql_nullable_number(activity.training_load),
            normalized_json = sql_text(&normalized_payload),
        ));
    }

    statements.push("COMMIT;".to_string());
    execute_sql(database_path, &statements.join("\n"))?;
    Ok(())
}

#[tauri::command]
fn bootstrap_local_store(app: AppHandle) -> Result<StorageSnapshot, String> {
    prepare_local_store(&app)
}

#[tauri::command]
fn load_login_session() -> Result<Option<LoginSession>, String> {
    Ok(load_session_from_keychain()?.map(|stored| stored.session))
}

#[tauri::command]
fn clear_login_session() -> Result<(), String> {
    clear_session_from_keychain()
}

#[tauri::command]
fn authenticate_garmin(
    credentials: LoginCredentials,
    runtime_state: State<GarminRuntimeState>,
) -> Result<GarminAuthResponse, String> {
    if !garmin_adapter_ready() {
        return Err(
            "The Garmin adapter is not ready. Run `npm run setup:garmin-adapter` first."
                .to_string(),
        );
    }

    {
        let mut pending = runtime_state
            .pending_mfa_challenge
            .lock()
            .map_err(|_| "Unable to lock the Garmin MFA runtime state.".to_string())?;
        *pending = None;
    }

    let output = run_python_adapter(
        "authenticate",
        &json!({
            "email": credentials.email.trim().to_lowercase(),
            "password": credentials.password,
        }),
    )?;

    let response: GarminAdapterAuthResponse = serde_json::from_str(&output)
        .map_err(|error| format!("Unable to parse the Garmin auth response: {error}"))?;

    match response {
        GarminAdapterAuthResponse::MfaRequired {
            message,
            challenge_state,
            account_email,
        } => {
            let mut pending = runtime_state
                .pending_mfa_challenge
                .lock()
                .map_err(|_| "Unable to lock the Garmin MFA runtime state.".to_string())?;
            *pending = Some(PendingMfaChallenge {
                account_email,
                challenge_state,
            });

            Ok(GarminAuthResponse::MfaRequired { message })
        }
        GarminAdapterAuthResponse::Authenticated {
            session,
            token_store,
        } => {
            store_session_in_keychain(&StoredSession {
                session: session.clone(),
                token_store,
            })?;

            Ok(GarminAuthResponse::Authenticated { session })
        }
    }
}

#[tauri::command]
fn resume_garmin_mfa(
    mfa_code: String,
    runtime_state: State<GarminRuntimeState>,
) -> Result<GarminAuthResponse, String> {
    let pending = {
        let mut guard = runtime_state
            .pending_mfa_challenge
            .lock()
            .map_err(|_| "Unable to lock the Garmin MFA runtime state.".to_string())?;
        guard
            .take()
            .ok_or_else(|| "No pending Garmin MFA challenge is available.".to_string())?
    };

    let output = run_python_adapter(
        "resume-mfa",
        &json!({
            "email": pending.account_email,
            "challengeState": pending.challenge_state,
            "mfaCode": mfa_code.trim(),
        }),
    )?;

    let response: GarminAdapterAuthResponse = serde_json::from_str(&output)
        .map_err(|error| format!("Unable to parse the Garmin MFA response: {error}"))?;

    match response {
        GarminAdapterAuthResponse::Authenticated {
            session,
            token_store,
        } => {
            store_session_in_keychain(&StoredSession {
                session: session.clone(),
                token_store,
            })?;

            Ok(GarminAuthResponse::Authenticated { session })
        }
        GarminAdapterAuthResponse::MfaRequired { message, .. } => {
            Err(format!("Garmin still requires MFA: {message}"))
        }
    }
}

#[tauri::command]
fn save_sync_summary(app: AppHandle, summary: SyncSummary) -> Result<(), String> {
    let paths = resolve_storage_paths(&app)?;
    ensure_storage_directories(&paths)?;
    write_sync_summary_to_disk(&paths, &summary)
}

#[tauri::command]
fn load_sync_summary(app: AppHandle) -> Result<Option<SyncSummary>, String> {
    let paths = resolve_storage_paths(&app)?;
    load_sync_summary_from_disk(&paths)
}

#[tauri::command]
fn clear_sync_summary(app: AppHandle) -> Result<(), String> {
    let paths = resolve_storage_paths(&app)?;
    clear_sync_summary_from_disk(&paths)
}

#[tauri::command]
fn sync_garmin_running_data(app: AppHandle) -> Result<SyncSummary, String> {
    let paths = resolve_storage_paths(&app)?;
    ensure_storage_directories(&paths)?;
    initialize_database(&paths.database_path)?;

    let stored_session = load_session_from_keychain()?.ok_or_else(|| {
        "No Garmin session is stored locally. Sign in before starting a sync.".to_string()
    })?;

    let output = run_python_adapter(
        "sync-running",
        &json!({
            "tokenStore": stored_session.token_store,
            "rawDataDir": paths.raw_data_dir.display().to_string(),
            "maxActivities": DEFAULT_SYNC_ACTIVITY_LIMIT,
            "accountEmail": stored_session.session.account_email,
        }),
    )?;

    let response: GarminAdapterSyncResponse = serde_json::from_str(&output)
        .map_err(|error| format!("Unable to parse the Garmin sync response: {error}"))?;

    store_session_in_keychain(&StoredSession {
        session: response.session.clone(),
        token_store: response.token_store,
    })?;

    persist_synced_activities(&paths.database_path, &response.session, &response.activities)?;

    let raw_activities = query_count(&paths.database_path, "activities_raw")?;
    let normalized_activities = query_count(&paths.database_path, "activities_normalized")?;
    let warning_suffix = if response.warnings.is_empty() {
        String::new()
    } else {
        format!(
            " {} activity detail requests reported warnings.",
            response.warnings.len()
        )
    };

    let summary = SyncSummary {
        last_synced_at: now_iso_string(),
        raw_activities,
        normalized_activities,
        status: "ready".to_string(),
        message: format!(
            "Imported {} recent running activities into the local raw store and SQLite analytics database.{}",
            response.activities.len(),
            warning_suffix
        ),
    };

    write_sync_summary_to_disk(&paths, &summary)?;
    Ok(summary)
}

pub fn run() {
    tauri::Builder::default()
        .manage(GarminRuntimeState::default())
        .setup(|app| {
            prepare_local_store(&app.handle())
                .map(|_| ())
                .map_err(|error| -> Box<dyn std::error::Error> { error.into() })?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap_local_store,
            authenticate_garmin,
            resume_garmin_mfa,
            load_login_session,
            clear_login_session,
            save_sync_summary,
            load_sync_summary,
            clear_sync_summary,
            sync_garmin_running_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
