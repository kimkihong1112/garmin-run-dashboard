CREATE TABLE IF NOT EXISTS athletes (
  id TEXT PRIMARY KEY,
  garmin_athlete_id TEXT UNIQUE,
  display_name TEXT NOT NULL,
  timezone TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS sync_runs (
  id TEXT PRIMARY KEY,
  started_at TEXT NOT NULL,
  finished_at TEXT,
  status TEXT NOT NULL,
  raw_count INTEGER NOT NULL DEFAULT 0,
  normalized_count INTEGER NOT NULL DEFAULT 0,
  error_message TEXT
);

CREATE TABLE IF NOT EXISTS activities_raw (
  id TEXT PRIMARY KEY,
  athlete_id TEXT NOT NULL,
  garmin_activity_id TEXT NOT NULL UNIQUE,
  activity_type TEXT NOT NULL,
  source_created_at TEXT NOT NULL,
  imported_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  payload_json TEXT NOT NULL,
  FOREIGN KEY (athlete_id) REFERENCES athletes (id)
);

CREATE TABLE IF NOT EXISTS activities_normalized (
  activity_id TEXT PRIMARY KEY,
  athlete_id TEXT NOT NULL,
  local_start_at TEXT NOT NULL,
  utc_start_at TEXT NOT NULL,
  distance_meters REAL NOT NULL,
  duration_seconds REAL NOT NULL,
  average_pace_seconds_per_km REAL,
  average_heart_rate REAL,
  max_heart_rate REAL,
  elevation_gain_meters REAL,
  training_load REAL,
  normalized_json TEXT NOT NULL,
  FOREIGN KEY (athlete_id) REFERENCES athletes (id)
);

CREATE TABLE IF NOT EXISTS dashboard_snapshots (
  granularity TEXT NOT NULL,
  bucket_start TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  generated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (granularity, bucket_start)
);

CREATE INDEX IF NOT EXISTS idx_activities_raw_athlete
  ON activities_raw (athlete_id, source_created_at);

CREATE INDEX IF NOT EXISTS idx_activities_normalized_athlete
  ON activities_normalized (athlete_id, local_start_at);
