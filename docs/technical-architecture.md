# Technical Architecture

## 1. Architecture Overview

The application is organized into five layers:

1. Presentation layer
2. Application services
3. Platform adapter layer
4. Local persistence layer
5. Analytics layer

This separation is important because Garmin authentication and sync behavior are fragile and may change independently from the local dashboards.

## 2. Chosen Stack

### Frontend

- React
- TypeScript
- Vite
- CSS with a custom design system

### Desktop Runtime

- Tauri 2
- Rust commands exposed through Tauri IPC

### Local Persistence

- SQLite for normalized data
- raw JSON files for source payload preservation
- JSON metadata snapshots for operational state
- platform keychain for encrypted session persistence

## 3. Module Boundaries

### 3.1 Frontend Modules

- `features/auth`: login and MFA flows
- `features/dashboard`: daily, weekly, monthly workspace presentation
- `components`: reusable view primitives such as segmented controls, charts, rings, and heatmaps
- `lib`: runtime bridges, storage helpers, mock adapters, and shared models

### 3.2 Backend Modules

- secure session vault
- local storage bootstrap
- database bootstrap
- sync state persistence
- future Garmin connector commands

## 4. Local Storage Design

### 4.1 Directory Layout

The app prepares these directories inside the application support path:

- `db/`
- `raw/`
- `normalized/`
- `meta/`

### 4.2 Data Separation

The system stores three forms of information:

1. Secret session data
2. Raw imported Garmin payloads
3. Normalized analytical records

Secret session data should not live in the same place as bulk activity data. In the initial scaffold, session data is stored in the macOS Keychain, while raw and normalized data live under the app data directory.

## 5. Database Design

The initial SQLite schema includes:

- `athletes`
- `sync_runs`
- `activities_raw`
- `activities_normalized`
- `dashboard_snapshots`

### 5.1 Why Keep Raw Data

Keeping raw payloads locally enables:

- auditability when numbers look wrong
- reprocessing when parsers change
- debugging login and sync regressions
- future derived metrics without re-fetching all source data

### 5.2 Why Keep Normalized Data

Normalized tables enable:

- fast filtering and aggregation
- versioned transformation logic
- consistent daily, weekly, and monthly summaries

## 6. Secure Session Strategy

### 6.1 Initial Implementation

The first scaffold stores serialized session payloads in the macOS Keychain by invoking the native `security` command from the Rust backend.

This is a pragmatic first step because:

- it avoids storing session tokens in plaintext files
- it avoids inventing custom encryption without a secure key source
- it matches the current macOS development environment

### 6.2 Cross-Platform Follow-Up

If Windows and Linux support are added, the same interface should be implemented through:

- Windows Credential Manager
- Secret Service / libsecret on Linux

## 7. Garmin Integration Strategy

Garmin Connect access must be isolated behind an adapter interface.

Recommended internal modules:

- `auth adapter`
- `activity fetch adapter`
- `payload mapper`
- `normalizer`
- `sync orchestrator`

This protects the rest of the app from:

- login flow changes
- MFA changes
- response format changes
- temporary sync failures

## 8. Dashboard Composition Strategy

The three dashboard ranges should not share identical layouts.

### Daily

- workout execution quality
- pace trend
- heart rate distribution
- split review

### Weekly

- mileage and time volume
- workout mix
- streak and consistency
- recovery rhythm

### Monthly

- cumulative progression
- long-run cadence
- best efforts and trend signals
- calendar density

## 9. UI System Strategy

The visual language should follow these rules:

- calm background hierarchy
- strong typography
- minimal chrome
- motion only where it improves orientation
- emphasis on scanability

The first version intentionally avoids a card-heavy SaaS dashboard style. Sections should feel like analytic surfaces rather than floating widgets.

## 10. Failure States

The product needs explicit UI support for these failures:

1. Garmin credentials rejected
2. MFA requested
3. network unavailable
4. sync partially completed
5. raw payload saved but normalization failed
6. database unavailable or corrupted
7. expired local session

## 11. Recommended Near-Term Backlog

1. Replace the mock Garmin adapter with a real sync implementation.
2. Add parser fixtures from saved raw payloads.
3. Materialize daily and weekly aggregates into SQLite.
4. Add filter controls for date range and workout type.
5. Add export and local data deletion flows.
