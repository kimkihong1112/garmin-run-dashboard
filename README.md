# Garmin Run Dashboard

Garmin Run Dashboard is a local-first desktop application for importing raw Garmin Connect running data, normalizing it on-device, and presenting daily, weekly, and monthly insights in a calm analytics workspace.

This repository currently includes:

- a rewritten English product requirements document
- a technical architecture document
- an initial Tauri + React + TypeScript app shell
- a login flow that expands for MFA challenges
- local storage foundations for secure session persistence and SQLite bootstrap

## Product Direction

- Platform: Desktop application
- App model: Local-first
- Security model: Session tokens stored in the macOS Keychain in the current scaffold
- Data model: Raw Garmin payloads plus normalized analytics tables stored locally
- UI direction: Inspired by Apple Fitness and Apple Health, but optimized for dense, readable running analytics

## Current Status

The Garmin integration is intentionally scaffolded behind an adapter boundary because Garmin Connect does not provide an official public API for this use case. The current build includes a mocked login/sync path so the secure storage, dashboard structure, and local data pipeline can be developed before the live connector is finalized.

## Quick Start

```bash
npm install
npm run tauri dev
```

For the current mock auth flow:

- any valid-looking email/password pair signs in
- an email containing `+mfa` will trigger the MFA expansion path

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
- The current scaffold focuses on the product foundation, secure local storage, and dashboard UX.
- Live Garmin sync, parsing, reconciliation, and data quality validation should be implemented next.
