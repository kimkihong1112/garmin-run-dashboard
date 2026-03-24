import { invoke } from "@tauri-apps/api/core";
import type {
  GarminAuthResult,
  LoginCredentials,
  LoginSession,
  SyncSummary,
} from "./models";
import { isTauriRuntime } from "./tauri";

export const MOCK_MFA_HINT =
  "Browser preview mode uses a local mock login. The desktop runtime uses the Python Garmin adapter.";

function delay(duration: number) {
  return new Promise((resolve) => {
    window.setTimeout(resolve, duration);
  });
}

function buildAthleteName(email: string) {
  const [handle] = email.split("@");
  return handle
    .replace(/\+.*$/, "")
    .split(/[._-]/)
    .filter(Boolean)
    .map((chunk) => chunk.charAt(0).toUpperCase() + chunk.slice(1))
    .join(" ");
}

function buildTokenLastFour(email: string) {
  const normalized = email.replace(/[^a-z0-9]/gi, "").toUpperCase();
  return normalized.slice(-4).padStart(4, "0");
}

export async function authenticateGarmin(
  credentials: LoginCredentials,
): Promise<GarminAuthResult> {
  if (isTauriRuntime()) {
    if (credentials.mfaCode?.trim()) {
      return invoke<GarminAuthResult>("resume_garmin_mfa", {
        mfaCode: credentials.mfaCode.trim(),
      });
    }

    return invoke<GarminAuthResult>("authenticate_garmin", {
      credentials: {
        email: credentials.email.trim(),
        password: credentials.password,
      },
    });
  }

  await delay(820);

  const email = credentials.email.trim().toLowerCase();
  const password = credentials.password.trim();
  const mfaCode = credentials.mfaCode?.trim() ?? "";

  if (!email || !password) {
    throw new Error("Enter both your Garmin email and password.");
  }

  if (!email.includes("@")) {
    throw new Error("Use a valid Garmin email address.");
  }

  // The live Garmin adapter has not been wired yet, so the current build uses
  // a deterministic mock challenge rule to exercise the MFA UI path.
  if (email.includes("+mfa") && !mfaCode) {
    return {
      status: "mfa_required",
      message:
        "Garmin requested a six-digit verification code. The form has been expanded.",
    };
  }

  if (mfaCode && !/^\d{6}$/.test(mfaCode)) {
    throw new Error("Verification codes must contain exactly 6 digits.");
  }

  const issuedAt = new Date();
  const expiresAt = new Date(issuedAt.getTime() + 1000 * 60 * 60 * 24 * 14);

  return {
    status: "authenticated",
    session: {
      athleteName: buildAthleteName(email) || "Garmin Runner",
      fullName: buildAthleteName(email) || "Garmin Runner",
      accountEmail: email,
      expiresAt: expiresAt.toISOString(),
      issuedAt: issuedAt.toISOString(),
      tokenLastFour: buildTokenLastFour(email),
      unitSystem: "metric",
    },
  };
}

export function buildInitialSyncSummary(): SyncSummary {
  return {
    lastSyncedAt: new Date(0).toISOString(),
    rawActivities: 0,
    normalizedActivities: 0,
    status: "idle",
    message:
      "No Garmin activities have been imported yet. Sign in to start the first local sync.",
  };
}

export async function syncGarminRunningData(): Promise<SyncSummary> {
  if (isTauriRuntime()) {
    return invoke<SyncSummary>("sync_garmin_running_data");
  }

  await delay(1200);

  return {
    lastSyncedAt: new Date().toISOString(),
    rawActivities: 24,
    normalizedActivities: 24,
    status: "ready",
    message:
      "Browser preview mode simulated a running sync. The desktop runtime will fetch live Garmin data.",
  };
}
