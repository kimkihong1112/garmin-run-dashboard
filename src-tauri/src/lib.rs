use chrono::{
    Datelike, Duration, NaiveDate, NaiveDateTime, SecondsFormat, Utc, Weekday,
};
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MetricStat {
    label: String,
    value: String,
    delta: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TrendPoint {
    label: String,
    primary: f64,
    accent: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DashboardRing {
    value: u8,
    label: String,
    caption: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DistributionSegment {
    label: String,
    value: u8,
    tone: String,
    detail: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ActivityTableRow {
    title: String,
    date: String,
    distance: String,
    pace: String,
    effort: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HeatmapCell {
    label: String,
    value: u32,
    level: u8,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DashboardScenario {
    eyebrow: String,
    title: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_empty: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    empty_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    empty_message: Option<String>,
    insight_title: String,
    insight: String,
    key_stats: Vec<MetricStat>,
    trend_title: String,
    trend_caption: String,
    trend: Vec<TrendPoint>,
    ring: DashboardRing,
    distribution_title: String,
    distribution: Vec<DistributionSegment>,
    activity_title: String,
    activities: Vec<ActivityTableRow>,
    notes_title: String,
    notes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    heatmap_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    heatmap: Option<Vec<HeatmapCell>>,
}

#[derive(Debug, Deserialize)]
struct NormalizedActivityRow {
    local_start_at: String,
    utc_start_at: String,
    distance_meters: f64,
    duration_seconds: f64,
    average_pace_seconds_per_km: Option<f64>,
    average_heart_rate: Option<f64>,
    max_heart_rate: Option<f64>,
    elevation_gain_meters: Option<f64>,
    training_load: Option<f64>,
    activity_name: Option<String>,
}

#[derive(Debug, Clone)]
struct ActivityRecord {
    activity_name: String,
    local_start_at: NaiveDateTime,
    utc_start_at: NaiveDateTime,
    distance_meters: f64,
    duration_seconds: f64,
    average_pace_seconds_per_km: Option<f64>,
    average_heart_rate: Option<f64>,
    max_heart_rate: Option<f64>,
    elevation_gain_meters: Option<f64>,
    training_load: Option<f64>,
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

fn parse_activity_datetime(value: &str) -> Option<NaiveDateTime> {
    if let Ok(timestamp) = chrono::DateTime::parse_from_rfc3339(value) {
        return Some(timestamp.naive_local());
    }

    for format in [
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M",
    ] {
        if let Ok(timestamp) = NaiveDateTime::parse_from_str(value, format) {
            return Some(timestamp);
        }
    }

    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        return date.and_hms_opt(0, 0, 0);
    }

    None
}

fn load_normalized_activities(
    database_path: &Path,
    limit: usize,
) -> Result<Vec<ActivityRecord>, String> {
    let query = format!(
        "SELECT \
            local_start_at, \
            utc_start_at, \
            distance_meters, \
            duration_seconds, \
            average_pace_seconds_per_km, \
            average_heart_rate, \
            max_heart_rate, \
            elevation_gain_meters, \
            training_load, \
            json_extract(normalized_json, '$.activityName') AS activity_name \
         FROM activities_normalized \
         ORDER BY utc_start_at DESC \
         LIMIT {};",
        limit
    );

    let output = Command::new("sqlite3")
        .arg("-json")
        .arg(database_path)
        .arg(query)
        .output()
        .map_err(|error| format!("Failed to query normalized activities with sqlite3: {error}"))?;

    if !output.status.success() {
        return Err(format!(
            "sqlite3 activity query failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("sqlite3 activity output was not valid UTF-8: {error}"))?;

    let rows: Vec<NormalizedActivityRow> = serde_json::from_str(stdout.trim())
        .map_err(|error| format!("Unable to parse normalized activity rows: {error}"))?;

    let mut activities = Vec::with_capacity(rows.len());

    for row in rows {
        let Some(local_start_at) = parse_activity_datetime(&row.local_start_at) else {
            continue;
        };
        let utc_start_at =
            parse_activity_datetime(&row.utc_start_at).unwrap_or(local_start_at);

        activities.push(ActivityRecord {
            activity_name: row
                .activity_name
                .unwrap_or_else(|| "Running activity".to_string()),
            local_start_at,
            utc_start_at,
            distance_meters: row.distance_meters,
            duration_seconds: row.duration_seconds,
            average_pace_seconds_per_km: row.average_pace_seconds_per_km,
            average_heart_rate: row.average_heart_rate,
            max_heart_rate: row.max_heart_rate,
            elevation_gain_meters: row.elevation_gain_meters,
            training_load: row.training_load,
        });
    }

    activities.sort_by(|left, right| right.utc_start_at.cmp(&left.utc_start_at));
    Ok(activities)
}

fn format_month_day(date: NaiveDate) -> String {
    date.format("%b %d").to_string()
}

fn format_month_year(date: NaiveDate) -> String {
    date.format("%B %Y").to_string()
}

fn format_distance_km(distance_meters: f64) -> String {
    format!("{:.1} km", distance_meters / 1000.0)
}

fn format_duration(duration_seconds: f64) -> String {
    let total_seconds = duration_seconds.max(0.0).round() as i64;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{hours}h {minutes:02}m")
    } else if minutes > 0 {
        format!("{minutes}m {seconds:02}s")
    } else {
        format!("{seconds}s")
    }
}

fn format_pace(value: Option<f64>) -> String {
    let Some(seconds_per_km) = value.filter(|pace| *pace > 0.0) else {
        return "—".to_string();
    };

    let total_seconds = seconds_per_km.round() as i64;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{minutes}:{seconds:02} /km")
}

fn format_hr(value: Option<f64>) -> String {
    value
        .map(|heart_rate| format!("{:.0} bpm", heart_rate))
        .unwrap_or_else(|| "—".to_string())
}

fn format_elevation(value: Option<f64>) -> String {
    value
        .map(|elevation| format!("{:.0} m", elevation))
        .unwrap_or_else(|| "—".to_string())
}

fn percentage_change(current: f64, previous: f64) -> Option<f64> {
    if previous.abs() < f64::EPSILON {
        None
    } else {
        Some(((current - previous) / previous) * 100.0)
    }
}

fn format_delta_percent(current: f64, previous: f64, fallback: &str) -> String {
    percentage_change(current, previous)
        .map(|delta| {
            if delta.abs() < 0.5 {
                "Flat versus the prior period".to_string()
            } else {
                format!("{:+.0}% versus the prior period", delta)
            }
        })
        .unwrap_or_else(|| fallback.to_string())
}

fn clamp_score(value: f64) -> u8 {
    value.round().clamp(0.0, 100.0) as u8
}

fn activity_bucket(activity: &ActivityRecord) -> &'static str {
    let distance_km = activity.distance_meters / 1000.0;
    let pace = activity.average_pace_seconds_per_km.unwrap_or(0.0);
    let load = activity.training_load.unwrap_or(0.0);

    if distance_km >= 18.0 {
        "Long run"
    } else if load >= 100.0 || (pace > 0.0 && pace <= 315.0) {
        "Quality"
    } else if distance_km >= 10.0 || (pace > 0.0 && pace <= 360.0) {
        "Steady"
    } else {
        "Easy"
    }
}

fn distribution_segments(activities: &[ActivityRecord]) -> Vec<DistributionSegment> {
    let mut easy = 0usize;
    let mut steady = 0usize;
    let mut quality = 0usize;
    let mut long = 0usize;

    for activity in activities {
        match activity_bucket(activity) {
            "Easy" => easy += 1,
            "Steady" => steady += 1,
            "Quality" => quality += 1,
            "Long run" => long += 1,
            _ => {}
        }
    }

    let total = activities.len().max(1) as f64;
    let make_segment = |label: &str, count: usize, tone: &str, detail: &str| DistributionSegment {
        label: label.to_string(),
        value: ((count as f64 / total) * 100.0).round() as u8,
        tone: tone.to_string(),
        detail: detail.to_string(),
    };

    vec![
        make_segment("Easy", easy, "#ffd6cb", "controlled aerobic mileage"),
        make_segment("Steady", steady, "#ffb59f", "moderate durable work"),
        make_segment("Quality", quality, "#ff6a48", "faster or heavier efforts"),
        make_segment("Long run", long, "#cb4026", "extended endurance sessions"),
    ]
}

fn build_activity_rows(activities: &[ActivityRecord], limit: usize) -> Vec<ActivityTableRow> {
    activities
        .iter()
        .take(limit)
        .map(|activity| ActivityTableRow {
            title: activity.activity_name.clone(),
            date: format_month_day(activity.local_start_at.date()),
            distance: format_distance_km(activity.distance_meters),
            pace: format_pace(activity.average_pace_seconds_per_km),
            effort: activity_bucket(activity).to_string(),
        })
        .collect()
}

fn average_distance(activities: &[ActivityRecord]) -> f64 {
    if activities.is_empty() {
        0.0
    } else {
        activities.iter().map(|activity| activity.distance_meters).sum::<f64>()
            / activities.len() as f64
    }
}

fn average_pace(activities: &[ActivityRecord]) -> Option<f64> {
    let values: Vec<f64> = activities
        .iter()
        .filter_map(|activity| activity.average_pace_seconds_per_km)
        .collect();

    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn average_training_load(activities: &[ActivityRecord]) -> f64 {
    let values: Vec<f64> = activities
        .iter()
        .filter_map(|activity| activity.training_load)
        .collect();

    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn build_empty_dashboard(range: &str) -> DashboardScenario {
    let (title, message) = match range {
        "daily" => (
            "The daily dashboard needs at least one synced run.",
            "Import Garmin activities to inspect the most recent workout, pacing, and training load on this machine.",
        ),
        "weekly" => (
            "The weekly dashboard needs recent running activity.",
            "Run a sync so the app can calculate seven-day volume, workout mix, and balance from local SQLite data.",
        ),
        _ => (
            "The monthly dashboard needs synced history.",
            "Import Garmin activities to unlock monthly progression charts, trend summaries, and the run-density heatmap.",
        ),
    };

    DashboardScenario {
        eyebrow: match range {
            "daily" => "Daily review".to_string(),
            "weekly" => "Weekly review".to_string(),
            _ => "Monthly review".to_string(),
        },
        title: "No running data available".to_string(),
        description: "The dashboard will switch from placeholder content to local analytics after the first successful sync.".to_string(),
        is_empty: Some(true),
        empty_title: Some(title.to_string()),
        empty_message: Some(message.to_string()),
        insight_title: "No primary insight yet.".to_string(),
        insight: "Sync Garmin activities to start generating local training analysis.".to_string(),
        key_stats: vec![],
        trend_title: "No trend data".to_string(),
        trend_caption: "Sync Garmin activities to populate this chart.".to_string(),
        trend: vec![],
        ring: DashboardRing {
            value: 0,
            label: "Readiness".to_string(),
            caption: "This score appears after local activity data is available.".to_string(),
        },
        distribution_title: "Workout mix".to_string(),
        distribution: vec![],
        activity_title: "Recent runs".to_string(),
        activities: vec![],
        notes_title: "Coach notes".to_string(),
        notes: vec![],
        heatmap_title: None,
        heatmap: None,
    }
}

fn build_daily_dashboard(activities: &[ActivityRecord]) -> DashboardScenario {
    let selected = &activities[0];
    let recent_runs = &activities[..activities.len().min(7)];
    let recent_baseline = &activities[1..activities.len().min(6)];
    let baseline_distance = average_distance(recent_baseline);
    let baseline_pace = average_pace(recent_baseline);
    let pace_delta = match (selected.average_pace_seconds_per_km, baseline_pace) {
        (Some(current), Some(previous)) if previous > 0.0 => {
            if current < previous {
                format!("{:.0}s per km quicker than recent average", previous - current)
            } else {
                format!("{:.0}s per km slower than recent average", current - previous)
            }
        }
        _ => "Baseline pace will sharpen with more history".to_string(),
    };

    let insight_title = if selected.training_load.unwrap_or(0.0) >= 100.0 {
        "The latest run landed as one of the heavier efforts in your recent block."
    } else if selected.distance_meters >= baseline_distance.max(1.0) {
        "The latest run stretched farther than your recent average."
    } else {
        "The latest run stayed in a controlled aerobic range."
    };

    let insight = format!(
        "{} covered {} in {} with an average pace of {} and an average heart rate of {}.",
        selected.activity_name,
        format_distance_km(selected.distance_meters),
        format_duration(selected.duration_seconds),
        format_pace(selected.average_pace_seconds_per_km),
        format_hr(selected.average_heart_rate),
    );

    let trend = recent_runs
        .iter()
        .rev()
        .map(|activity| TrendPoint {
            label: activity.local_start_at.date().format("%m/%d").to_string(),
            primary: activity
                .average_pace_seconds_per_km
                .map(|pace| pace / 60.0)
                .unwrap_or_else(|| (activity.distance_meters / 1000.0).max(0.1)),
            accent: activity.average_heart_rate,
        })
        .collect();

    let load_score = clamp_score(
        selected.training_load.unwrap_or(0.0) * 0.6
            + (selected.distance_meters / 1000.0) * 2.5
            + selected.elevation_gain_meters.unwrap_or(0.0) * 0.04,
    );

    DashboardScenario {
        eyebrow: "Daily review".to_string(),
        title: format!(
            "{} on {}",
            selected.activity_name,
            format_month_day(selected.local_start_at.date())
        ),
        description: "Review the latest synced run from the local Garmin archive and compare it against the recent baseline.".to_string(),
        is_empty: None,
        empty_title: None,
        empty_message: None,
        insight_title: insight_title.to_string(),
        insight,
        key_stats: vec![
            MetricStat {
                label: "Distance".to_string(),
                value: format_distance_km(selected.distance_meters),
                delta: if baseline_distance > 0.0 {
                    format!(
                        "{:+.1} km versus recent average",
                        (selected.distance_meters - baseline_distance) / 1000.0
                    )
                } else {
                    "Latest imported run".to_string()
                },
            },
            MetricStat {
                label: "Duration".to_string(),
                value: format_duration(selected.duration_seconds),
                delta: "Moving time for the selected run".to_string(),
            },
            MetricStat {
                label: "Avg pace".to_string(),
                value: format_pace(selected.average_pace_seconds_per_km),
                delta: pace_delta,
            },
            MetricStat {
                label: "Avg HR".to_string(),
                value: format_hr(selected.average_heart_rate),
                delta: format!(
                    "Max recorded HR {}",
                    format_hr(selected.max_heart_rate)
                ),
            },
        ],
        trend_title: "Recent pace trend".to_string(),
        trend_caption: "Primary line shows average pace across the latest runs. Accent line shows average heart rate.".to_string(),
        trend,
        ring: DashboardRing {
            value: load_score,
            label: "Workout load".to_string(),
            caption: format!(
                "This score blends distance, elevation, and training load from the selected run. {}",
                if load_score >= 75 {
                    "It reads as a substantial session relative to the recent pattern."
                } else {
                    "It reads as a controlled day rather than a major strain spike."
                }
            ),
        },
        distribution_title: "Recent run mix".to_string(),
        distribution: distribution_segments(&activities[..activities.len().min(14)]),
        activity_title: "Latest imported runs".to_string(),
        activities: build_activity_rows(activities, 6),
        notes_title: "Execution notes".to_string(),
        notes: vec![
            format!(
                "The latest effort was classified as {} based on distance, pace, and load.",
                activity_bucket(selected)
            ),
            format!(
                "Average pace currently sits at {} while the recent baseline sits near {}.",
                format_pace(selected.average_pace_seconds_per_km),
                format_pace(baseline_pace)
            ),
            format!(
                "Elevation gain reached {}, which helps explain the overall load score.",
                format_elevation(selected.elevation_gain_meters)
            ),
        ],
        heatmap_title: None,
        heatmap: None,
    }
}

fn build_weekly_dashboard(activities: &[ActivityRecord]) -> DashboardScenario {
    let anchor_date = activities[0].local_start_at.date();
    let start_date = anchor_date - Duration::days(6);
    let previous_start = start_date - Duration::days(7);
    let previous_end = start_date - Duration::days(1);

    let current_week: Vec<ActivityRecord> = activities
        .iter()
        .filter(|activity| {
            let date = activity.local_start_at.date();
            date >= start_date && date <= anchor_date
        })
        .cloned()
        .collect();

    let previous_week: Vec<ActivityRecord> = activities
        .iter()
        .filter(|activity| {
            let date = activity.local_start_at.date();
            date >= previous_start && date <= previous_end
        })
        .cloned()
        .collect();

    let current_distance = current_week.iter().map(|activity| activity.distance_meters).sum::<f64>();
    let previous_distance = previous_week.iter().map(|activity| activity.distance_meters).sum::<f64>();
    let current_duration = current_week.iter().map(|activity| activity.duration_seconds).sum::<f64>();
    let active_days = current_week
        .iter()
        .map(|activity| activity.local_start_at.date())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let long_run = current_week
        .iter()
        .max_by(|left, right| left.distance_meters.total_cmp(&right.distance_meters))
        .cloned();
    let quality_count = current_week
        .iter()
        .filter(|activity| activity_bucket(activity) == "Quality")
        .count();

    let mut trend = Vec::with_capacity(7);

    for day_offset in 0..7 {
        let day = start_date + Duration::days(day_offset);
        let day_runs: Vec<&ActivityRecord> = current_week
            .iter()
            .filter(|activity| activity.local_start_at.date() == day)
            .collect();

        let distance = day_runs
            .iter()
            .map(|activity| activity.distance_meters)
            .sum::<f64>()
            / 1000.0;
        let load = if day_runs.is_empty() {
            0.0
        } else {
            day_runs
                .iter()
                .filter_map(|activity| activity.training_load)
                .sum::<f64>()
                / day_runs.len() as f64
        };

        trend.push(TrendPoint {
            label: day.format("%a").to_string(),
            primary: distance,
            accent: Some(load),
        });
    }

    let balance_score = clamp_score(
        48.0 + active_days as f64 * 7.0 + if long_run.is_some() { 6.0 } else { 0.0 }
            - (quality_count.saturating_sub(2) as f64 * 10.0)
            - if active_days == 7 { 8.0 } else { 0.0 },
    );

    DashboardScenario {
        eyebrow: "Weekly review".to_string(),
        title: format!("Week ending {}", format_month_day(anchor_date)),
        description: "Track seven-day volume, workout mix, and how evenly the local training load was distributed across the week.".to_string(),
        is_empty: None,
        empty_title: None,
        empty_message: None,
        insight_title: if quality_count >= 3 {
            "The week carried several higher-intensity touches."
        } else if active_days <= 3 {
            "The week was light on frequency and depended on a few larger runs."
        } else {
            "The week stayed reasonably balanced across volume and recovery."
        }
        .to_string(),
        insight: format!(
            "This seven-day block included {} runs across {} active days for a total of {} and {} of moving time.",
            current_week.len(),
            active_days,
            format_distance_km(current_distance),
            format_duration(current_duration),
        ),
        key_stats: vec![
            MetricStat {
                label: "Distance".to_string(),
                value: format_distance_km(current_distance),
                delta: format_delta_percent(
                    current_distance,
                    previous_distance,
                    "No prior week available yet",
                ),
            },
            MetricStat {
                label: "Time on feet".to_string(),
                value: format_duration(current_duration),
                delta: "Seven-day moving total".to_string(),
            },
            MetricStat {
                label: "Runs".to_string(),
                value: format!("{} sessions", current_week.len()),
                delta: format!("{} active days", active_days),
            },
            MetricStat {
                label: "Longest run".to_string(),
                value: long_run
                    .as_ref()
                    .map(|activity| format_distance_km(activity.distance_meters))
                    .unwrap_or_else(|| "—".to_string()),
                delta: long_run
                    .as_ref()
                    .map(|activity| format!("{} on {}", activity.activity_name, format_month_day(activity.local_start_at.date())))
                    .unwrap_or_else(|| "No long run captured".to_string()),
            },
        ],
        trend_title: "Volume by day".to_string(),
        trend_caption: "Primary line shows kilometers by day. Accent line shows average training load for that day.".to_string(),
        trend,
        ring: DashboardRing {
            value: balance_score,
            label: "Weekly balance".to_string(),
            caption: if balance_score >= 75 {
                "The week looks balanced, with enough spread across the days to avoid leaning on a single run.".to_string()
            } else {
                "The week clustered stress into a narrower set of days, so the next block may benefit from smoother spacing.".to_string()
            },
        },
        distribution_title: "Workout mix".to_string(),
        distribution: distribution_segments(&current_week),
        activity_title: "Runs captured this week".to_string(),
        activities: build_activity_rows(&current_week, 6),
        notes_title: "Weekly coaching notes".to_string(),
        notes: vec![
            format!("The week logged {} active days, leaving {} true rest days.", active_days, 7usize.saturating_sub(active_days)),
            format!("Average training load per run landed around {:.0}.", average_training_load(&current_week)),
            match long_run {
                Some(activity) => format!(
                    "The longest outing was {} at {}.",
                    activity.activity_name,
                    format_distance_km(activity.distance_meters)
                ),
                None => "No single long run stood out in this block.".to_string(),
            },
        ],
        heatmap_title: None,
        heatmap: None,
    }
}

fn build_monthly_heatmap(activities: &[ActivityRecord], month_start: NaiveDate, month_end: NaiveDate) -> Vec<HeatmapCell> {
    let mut grid_start = month_start;
    while grid_start.weekday() != Weekday::Mon {
        grid_start -= Duration::days(1);
    }

    let mut grid_end = month_end;
    while grid_end.weekday() != Weekday::Sun {
        grid_end += Duration::days(1);
    }

    let max_distance = activities
        .iter()
        .map(|activity| activity.distance_meters / 1000.0)
        .fold(0.0_f64, f64::max);

    let mut cells = Vec::new();
    let mut current = grid_start;

    while current <= grid_end {
        let day_distance = activities
            .iter()
            .filter(|activity| activity.local_start_at.date() == current)
            .map(|activity| activity.distance_meters / 1000.0)
            .sum::<f64>();

        let level = if day_distance <= 0.0 || max_distance <= 0.0 {
            0
        } else {
            ((day_distance / max_distance) * 5.0).ceil().clamp(1.0, 5.0) as u8
        };

        cells.push(HeatmapCell {
            label: format_month_day(current),
            value: day_distance.round() as u32,
            level,
        });

        current += Duration::days(1);
    }

    cells
}

fn build_monthly_dashboard(activities: &[ActivityRecord]) -> DashboardScenario {
    let anchor_date = activities[0].local_start_at.date();
    let month_start = anchor_date.with_day(1).unwrap_or(anchor_date);
    let next_month = if anchor_date.month() == 12 {
        NaiveDate::from_ymd_opt(anchor_date.year() + 1, 1, 1).unwrap_or(anchor_date)
    } else {
        NaiveDate::from_ymd_opt(anchor_date.year(), anchor_date.month() + 1, 1)
            .unwrap_or(anchor_date)
    };
    let month_end = next_month - Duration::days(1);

    let current_month: Vec<ActivityRecord> = activities
        .iter()
        .filter(|activity| {
            let date = activity.local_start_at.date();
            date >= month_start && date <= month_end
        })
        .cloned()
        .collect();

    let previous_month_end = month_start - Duration::days(1);
    let previous_month_start = previous_month_end.with_day(1).unwrap_or(previous_month_end);
    let previous_month: Vec<ActivityRecord> = activities
        .iter()
        .filter(|activity| {
            let date = activity.local_start_at.date();
            date >= previous_month_start && date <= previous_month_end
        })
        .cloned()
        .collect();

    let current_distance = current_month.iter().map(|activity| activity.distance_meters).sum::<f64>();
    let previous_distance = previous_month.iter().map(|activity| activity.distance_meters).sum::<f64>();
    let longest_run = current_month
        .iter()
        .max_by(|left, right| left.distance_meters.total_cmp(&right.distance_meters))
        .cloned();
    let best_pace = current_month
        .iter()
        .filter(|activity| activity.distance_meters >= 5000.0)
        .filter_map(|activity| activity.average_pace_seconds_per_km)
        .min_by(|left, right| left.total_cmp(right));

    let weekly_buckets = (1u32..=5)
        .map(|bucket| {
            let bucket_runs: Vec<&ActivityRecord> = current_month
                .iter()
                .filter(|activity| ((activity.local_start_at.date().day() - 1) / 7) + 1 == bucket)
                .collect();
            let distance = bucket_runs
                .iter()
                .map(|activity| activity.distance_meters)
                .sum::<f64>()
                / 1000.0;
            let load = if bucket_runs.is_empty() {
                0.0
            } else {
                bucket_runs
                    .iter()
                    .filter_map(|activity| activity.training_load)
                    .sum::<f64>()
                    / bucket_runs.len() as f64
            };

            TrendPoint {
                label: format!("W{bucket}"),
                primary: distance,
                accent: Some(load),
            }
        })
        .collect::<Vec<_>>();

    let active_days = current_month
        .iter()
        .map(|activity| activity.local_start_at.date())
        .collect::<std::collections::BTreeSet<_>>()
        .len();

    let block_quality = clamp_score(
        42.0 + active_days as f64 * 1.8 + current_month.len() as f64 * 1.4
            + longest_run
                .as_ref()
                .map(|activity| activity.distance_meters / 1000.0 * 0.8)
                .unwrap_or(0.0),
    );

    DashboardScenario {
        eyebrow: "Monthly review".to_string(),
        title: format!("{} overview", format_month_year(anchor_date)),
        description: "Step back and review cumulative volume, repeatability, and the shape of the local running block across the month.".to_string(),
        is_empty: None,
        empty_title: None,
        empty_message: None,
        insight_title: if longest_run
            .as_ref()
            .map(|activity| activity.distance_meters >= 20000.0)
            .unwrap_or(false)
        {
            "The month established a durable long-run backbone."
        } else if current_month.len() >= 12 {
            "The month emphasized frequency and steady repeatability."
        } else {
            "The month remained lighter and more selective in total volume."
        }
        .to_string(),
        insight: format!(
            "The local archive captured {} runs across {} active days for a total of {} in {}.",
            current_month.len(),
            active_days,
            format_distance_km(current_distance),
            format_month_year(anchor_date),
        ),
        key_stats: vec![
            MetricStat {
                label: "Distance".to_string(),
                value: format_distance_km(current_distance),
                delta: format_delta_percent(
                    current_distance,
                    previous_distance,
                    "No prior month available yet",
                ),
            },
            MetricStat {
                label: "Runs".to_string(),
                value: format!("{} sessions", current_month.len()),
                delta: format!("{} active days", active_days),
            },
            MetricStat {
                label: "Longest run".to_string(),
                value: longest_run
                    .as_ref()
                    .map(|activity| format_distance_km(activity.distance_meters))
                    .unwrap_or_else(|| "—".to_string()),
                delta: longest_run
                    .as_ref()
                    .map(|activity| format!("{} on {}", activity.activity_name, format_month_day(activity.local_start_at.date())))
                    .unwrap_or_else(|| "No standout long run".to_string()),
            },
            MetricStat {
                label: "Best pace".to_string(),
                value: format_pace(best_pace),
                delta: "Fastest average pace from runs longer than 5 km".to_string(),
            },
        ],
        trend_title: "Weekly mileage across the month".to_string(),
        trend_caption: "Primary line shows weekly kilometers. Accent line shows average load per run in that week bucket.".to_string(),
        trend: weekly_buckets,
        ring: DashboardRing {
            value: block_quality,
            label: "Block quality".to_string(),
            caption: if block_quality >= 78 {
                "The month reads as a coherent running block with enough frequency to build momentum.".to_string()
            } else {
                "The month stayed relatively light, which may be appropriate if this was a transition or recovery block.".to_string()
            },
        },
        distribution_title: "Monthly run mix".to_string(),
        distribution: distribution_segments(&current_month),
        activity_title: "Monthly signature runs".to_string(),
        activities: build_activity_rows(&current_month, 6),
        notes_title: "Month-end notes".to_string(),
        notes: vec![
            format!(
                "The current month logged {} runs with an average distance of {:.1} km.",
                current_month.len(),
                if current_month.is_empty() {
                    0.0
                } else {
                    (current_distance / current_month.len() as f64) / 1000.0
                }
            ),
            match longest_run {
                Some(activity) => format!(
                    "The longest outing reached {} and anchors the month.",
                    format_distance_km(activity.distance_meters)
                ),
                None => "No long run anchor was found in this month.".to_string(),
            },
            format!(
                "The monthly view currently compares against {} stored runs from the prior month.",
                previous_month.len()
            ),
        ],
        heatmap_title: Some("Run density calendar".to_string()),
        heatmap: Some(build_monthly_heatmap(&current_month, month_start, month_end)),
    }
}

fn build_dashboard_scenario(range: &str, activities: &[ActivityRecord]) -> DashboardScenario {
    if activities.is_empty() {
        return build_empty_dashboard(range);
    }

    match range {
        "daily" => build_daily_dashboard(activities),
        "weekly" => build_weekly_dashboard(activities),
        "monthly" => build_monthly_dashboard(activities),
        _ => build_daily_dashboard(activities),
    }
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

#[tauri::command]
fn load_dashboard_scenario(app: AppHandle, range: String) -> Result<DashboardScenario, String> {
    let paths = resolve_storage_paths(&app)?;
    ensure_storage_directories(&paths)?;
    initialize_database(&paths.database_path)?;

    let activities = load_normalized_activities(&paths.database_path, 400)?;
    Ok(build_dashboard_scenario(&range, &activities))
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
            load_dashboard_scenario,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_activity(
        year: i32,
        month: u32,
        day: u32,
        distance_meters: f64,
        duration_seconds: f64,
        average_pace_seconds_per_km: f64,
        average_heart_rate: f64,
        training_load: f64,
        name: &str,
    ) -> ActivityRecord {
        let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
        let timestamp = date.and_hms_opt(6, 30, 0).unwrap();

        ActivityRecord {
            activity_name: name.to_string(),
            local_start_at: timestamp,
            utc_start_at: timestamp,
            distance_meters,
            duration_seconds,
            average_pace_seconds_per_km: Some(average_pace_seconds_per_km),
            average_heart_rate: Some(average_heart_rate),
            max_heart_rate: Some(average_heart_rate + 12.0),
            elevation_gain_meters: Some(55.0),
            training_load: Some(training_load),
        }
    }

    #[test]
    fn empty_dashboard_is_returned_when_no_activities_exist() {
        let scenario = build_dashboard_scenario("daily", &[]);

        assert_eq!(scenario.is_empty, Some(true));
        assert!(scenario.empty_title.is_some());
        assert!(scenario.key_stats.is_empty());
    }

    #[test]
    fn weekly_dashboard_aggregates_recent_runs() {
        let activities = vec![
            sample_activity(2026, 3, 24, 12000.0, 3200.0, 266.0, 151.0, 88.0, "Tempo"),
            sample_activity(2026, 3, 23, 8000.0, 2700.0, 337.0, 139.0, 46.0, "Easy"),
            sample_activity(2026, 3, 21, 21000.0, 6200.0, 295.0, 148.0, 112.0, "Long"),
            sample_activity(2026, 3, 18, 9000.0, 3000.0, 333.0, 140.0, 52.0, "Prior week"),
        ];

        let scenario = build_weekly_dashboard(&activities);

        assert_eq!(scenario.eyebrow, "Weekly review");
        assert_eq!(scenario.key_stats[0].value, "50.0 km");
        assert_eq!(scenario.trend.len(), 7);
        assert!(!scenario.activities.is_empty());
    }

    #[test]
    fn monthly_dashboard_builds_heatmap_and_signature_runs() {
        let activities = vec![
            sample_activity(2026, 3, 29, 28000.0, 8200.0, 293.0, 149.0, 124.0, "Long run"),
            sample_activity(2026, 3, 24, 12000.0, 3300.0, 275.0, 154.0, 95.0, "Tempo"),
            sample_activity(2026, 3, 19, 10000.0, 2550.0, 255.0, 162.0, 108.0, "10K effort"),
            sample_activity(2026, 3, 11, 13600.0, 3680.0, 271.0, 150.0, 101.0, "Cruise intervals"),
            sample_activity(2026, 2, 20, 9000.0, 3100.0, 344.0, 138.0, 51.0, "Previous month"),
        ];

        let scenario = build_monthly_dashboard(&activities);

        assert_eq!(scenario.eyebrow, "Monthly review");
        assert_eq!(scenario.key_stats[0].value, "63.6 km");
        assert!(scenario.heatmap.is_some());
        assert!(!scenario.activities.is_empty());
    }
}
