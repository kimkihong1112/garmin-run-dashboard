# Product Requirements Document

## 1. Product Summary

Garmin Run Dashboard is a desktop analytics application that signs into Garmin Connect, imports raw running data to the local machine, transforms that data into a queryable analytics model, and presents daily, weekly, and monthly dashboards that help runners understand training load, performance, recovery, and consistency at a glance.

The product is intentionally local-first:

- raw Garmin responses stay on the user's machine
- normalized analytics tables stay on the user's machine
- session state is securely persisted locally
- the app continues to be useful offline after data sync

## 2. Product Goals

1. Import and preserve raw Garmin Connect running data locally.
2. Build trustworthy analytics views for daily, weekly, and monthly review.
3. Provide a secure desktop login flow with support for multi-factor authentication.
4. Make dense training data easy to understand in one glance.
5. Keep the system maintainable, public, and well documented in English.

## 3. Non-Goals For The First Delivery

1. Social features, coaching plans, or shared leaderboards.
2. Cloud sync across multiple user devices.
3. Editing Garmin data upstream.
4. Real-time watch sync over Bluetooth.
5. Support for non-running activities beyond what is needed for recovery context.

## 4. Primary User

- A runner who already records workouts in Garmin Connect and wants deeper personal analytics than Garmin's default web experience.

## 5. Core User Stories

1. As a runner, I can sign into Garmin Connect from the desktop app.
2. As a runner, I can complete MFA when Garmin asks for it.
3. As a runner, I can keep my session locally so I do not need to log in every time.
4. As a runner, I can sync my historical running data and keep both raw and normalized copies locally.
5. As a runner, I can review a daily dashboard for a single training day or workout.
6. As a runner, I can review a weekly dashboard for volume, distribution, and consistency.
7. As a runner, I can review a monthly dashboard for long-term progress and trends.
8. As a runner, I can trust that my private training data stays on my machine.

## 6. Functional Requirements

### 6.1 Authentication

1. The landing screen must be a login screen only.
2. The login screen must support Garmin email/username and password.
3. If Garmin requests a second factor, the login form must expand to reveal a verification code field.
4. Authentication state must clearly show loading, error, and challenge-required states.
5. Login session data must be securely stored locally after successful authentication.
6. The user must be able to sign out and clear the stored session.

### 6.2 Local-First Storage

1. Raw Garmin responses must be stored locally for traceability and reprocessing.
2. Normalized data must be stored locally in a structured database.
3. Dashboard summaries may be materialized locally for fast loading.
4. Storage preparation must happen automatically when the app boots.
5. The product must make it clear that the primary source of truth is local.

### 6.3 Data Sync

1. The app must support an initial historical import.
2. The app must support incremental sync for newly completed activities.
3. Sync metadata must record started time, finished time, status, and item counts.
4. The sync layer must separate network fetching from parsing and normalization.
5. The app must preserve enough raw metadata to rebuild normalized tables if the parser changes.

### 6.4 Dashboards

1. The user must be able to switch between daily, weekly, and monthly views after login.
2. Each dashboard must be intentionally different instead of repeating the same widgets at a different date range.
3. Daily view should emphasize the selected workout, pacing, heart rate behavior, terrain, and execution quality.
4. Weekly view should emphasize mileage, time on feet, workout mix, streaks, and training balance.
5. Monthly view should emphasize progression, long-run rhythm, PR signals, and cumulative adaptation.
6. The dashboard UI must communicate the freshness of the currently displayed data.

### 6.5 UX And Presentation

1. The UI should feel modern, clean, and calm.
2. The UX should be inspired by Apple Fitness and Apple Health without copying them literally.
3. The data should be understandable with quick scanning.
4. The interface should work well on typical laptop and desktop resolutions.
5. The visual system should support long-form analytical reading, not only glanceable cards.

### 6.6 Engineering Process

1. The repository should be public on GitHub.
2. Documentation should be written in English.
3. Source code comments should be written in English.
4. Development should progress through detailed milestone commits.

## 7. Recommended Technical Stack

### 7.1 Desktop Platform

- Tauri 2
- Rust backend
- React + TypeScript frontend
- Vite for frontend development

Why:

- smaller desktop footprint than Electron
- strong local filesystem control
- easier secure native integrations
- good fit for SQLite and platform keychain access

### 7.2 Local Data Layer

- SQLite for normalized tables and materialized aggregates
- raw JSON storage for unmodified Garmin payloads
- JSON metadata files for sync state snapshots where convenient

### 7.3 Security Layer

- macOS Keychain for session token storage in the first implementation
- adapter-based path for Windows Credential Manager and Linux Secret Service later

### 7.4 Frontend UX Layer

- React component architecture
- custom SVG-based charts in the first version for lightweight control
- a restrained visual system instead of a dashboard-card mosaic

## 8. Missing Requirements And Recommended Additions

The original request is strong on intent, but several requirements need to be made explicit to avoid expensive rework.

### 8.1 Platform Scope

Missing:

- whether version 1 targets macOS only or multiple desktop OSes

Recommendation:

- ship macOS first because the current environment is macOS and it enables Keychain-backed secure storage quickly

### 8.2 Garmin Integration Strategy

Missing:

- how Garmin authentication will be handled if the login flow changes
- what happens when CAPTCHA, device approval, or additional risk checks appear

Recommendation:

- treat Garmin integration as an adapter with clear interfaces and fallback error states
- document that Garmin Connect access relies on an unofficial integration surface

### 8.3 Data Freshness Rules

Missing:

- whether sync occurs automatically on app open
- how often background refresh should run
- whether manual re-sync should be exposed

Recommendation:

- include manual sync in version 1
- support optional auto-sync on app launch later

### 8.4 Activity Scope

Missing:

- whether only runs are stored or whether cross-training should be imported for context

Recommendation:

- import all activities into raw storage
- normalize only running and recovery-relevant metadata in version 1

### 8.5 Timezone And Calendar Rules

Missing:

- how to define day/week/month boundaries for travelers or workouts around midnight

Recommendation:

- store timestamps in UTC
- compute dashboard buckets in the athlete's preferred local timezone

### 8.6 Privacy And Recovery

Missing:

- whether data export, backup, and full local deletion are required

Recommendation:

- add export and delete-local-data requirements before beta

### 8.7 Quality And Testing

Missing:

- expected test coverage for parsing, normalization, and analytics correctness

Recommendation:

- require fixture-based parser tests
- require analytics regression snapshots for daily, weekly, and monthly aggregations

## 9. Risks

1. Garmin Connect is not an officially supported public API for this workflow.
2. MFA and login risk checks may change without notice.
3. Raw data quality may differ across historical activities and device generations.
4. Dashboard trust depends on normalization accuracy and timezone handling.
5. Public open-source distribution may increase the need for careful trademark language and integration disclaimers.

## 10. Delivery Phases

### Phase 0: Foundation

- finalize requirements
- initialize repository
- scaffold desktop shell
- establish secure session storage
- bootstrap local database and raw data folders

### Phase 1: Auth And Local Platform

- implement Garmin login adapter
- implement MFA challenge handling
- store session securely
- expose storage/sync status in UI

### Phase 2: Sync Pipeline

- fetch historical activities
- store raw payloads
- normalize running activities
- track sync runs and reconciliation

### Phase 3: Dashboards

- implement daily dashboard
- implement weekly dashboard
- implement monthly dashboard
- add filtering and drill-down

### Phase 4: Hardening

- add parser fixtures
- add analytics regression tests
- add export/delete flows
- optimize performance for large histories
