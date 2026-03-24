import { invoke } from "@tauri-apps/api/core";
import type { LoginSession, StorageSnapshot, SyncSummary } from "./models";
import { isTauriRuntime } from "./tauri";

const SESSION_KEY = "garmin-run-dashboard:session";
const SYNC_KEY = "garmin-run-dashboard:sync-summary";

function parseJson<T>(value: string | null): T | null {
  if (!value) {
    return null;
  }

  try {
    return JSON.parse(value) as T;
  } catch {
    return null;
  }
}

export async function bootstrapLocalStore(): Promise<StorageSnapshot | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  return invoke<StorageSnapshot>("bootstrap_local_store");
}

export async function persistLoginSession(session: LoginSession) {
  if (isTauriRuntime()) {
    return;
  }

  // The browser fallback exists only to keep the UI preview usable outside the
  // desktop shell. Production secret storage should stay inside the backend.
  localStorage.setItem(SESSION_KEY, JSON.stringify(session));
}

export async function loadLoginSession(): Promise<LoginSession | null> {
  if (isTauriRuntime()) {
    return invoke<LoginSession | null>("load_login_session");
  }

  return parseJson<LoginSession>(localStorage.getItem(SESSION_KEY));
}

export async function clearLoginSession() {
  if (isTauriRuntime()) {
    await invoke("clear_login_session");
    return;
  }

  localStorage.removeItem(SESSION_KEY);
}

export async function persistSyncSummary(summary: SyncSummary) {
  if (isTauriRuntime()) {
    await invoke("save_sync_summary", { summary });
    return;
  }

  localStorage.setItem(SYNC_KEY, JSON.stringify(summary));
}

export async function loadSyncSummary(): Promise<SyncSummary | null> {
  if (isTauriRuntime()) {
    return invoke<SyncSummary | null>("load_sync_summary");
  }

  return parseJson<SyncSummary>(localStorage.getItem(SYNC_KEY));
}

export async function clearSyncSummary() {
  if (isTauriRuntime()) {
    await invoke("clear_sync_summary");
    return;
  }

  localStorage.removeItem(SYNC_KEY);
}
