# Garmin Run Dashboard

Garmin Run Dashboard is a local-first desktop application for importing raw Garmin Connect running data, normalizing it on-device, and presenting daily, weekly, and monthly insights in a calm analytics workspace.

This repository currently includes:

- a rewritten English product requirements document
- a technical architecture document
- an initial Tauri + React + TypeScript app shell
- a Garmin Connect adapter bridge using Python, `garth`, and `garminconnect`
- a login flow that expands for MFA challenges
- local storage foundations for secure session persistence, raw JSON ingestion, and SQLite bootstrap

## Product Direction

- Platform: Desktop application
- App model: Local-first
- Security model: Session tokens stored in the macOS Keychain in the current scaffold
- Data model: Raw Garmin payloads plus normalized analytics tables stored locally
- UI direction: Inspired by Apple Fitness and Apple Health, but optimized for dense, readable running analytics

## Current Status

The Garmin integration is intentionally isolated behind an adapter boundary because Garmin Connect does not provide an official public API for this use case. The current build uses a local Python adapter built on top of `garth` and `garminconnect`, while the Tauri backend remains responsible for secure token storage, raw file persistence, and SQLite normalization.

## Quick Start

```bash
npm install
npm run setup:garmin-adapter
npm run tauri dev
```

## Garmin Adapter Setup

The live Garmin adapter requires Python 3.12+ because the current `garth` and `garminconnect` packages target Python 3.10 or newer.

The setup script creates a local virtualenv at `.venv-garmin` and installs the adapter dependencies:

```bash
npm run setup:garmin-adapter
```

If you prefer to do it manually:

```bash
/opt/homebrew/bin/python3.12 -m venv .venv-garmin
.venv-garmin/bin/pip install garminconnect
```

## Project Structure

```text
docs/
  product-requirements.md
  technical-architecture.md
src/
  components/
  features/
  lib/
  styles/
scripts/
src-tauri/
  migrations/
  src/
```

## Local Storage Layout

The Tauri backend prepares the following local storage structure inside the application support directory:

- `db/garmin-run-dashboard.sqlite3`
- `raw/`
- `normalized/`
- `meta/last-sync-summary.json`

Sensitive login session payloads are stored in the macOS Keychain rather than inside SQLite.

## Notes

- All documentation and code comments are written in English for public collaboration.
- Raw Garmin activity payloads are written to local JSON files under the app data directory during sync.
- The current sync path stores normalized running summaries in SQLite and preserves detailed raw JSON on disk for future reprocessing.
