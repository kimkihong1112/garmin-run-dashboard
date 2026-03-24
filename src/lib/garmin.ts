import type { LoginCredentials, LoginSession, SyncSummary } from "./models";

export const MOCK_MFA_HINT =
  "Developer note: use an email containing +mfa to preview the MFA challenge path in this scaffold.";

interface GarminAuthChallenge {
  status: "mfa_required";
  message: string;
}

interface GarminAuthSuccess {
  status: "authenticated";
  session: LoginSession;
}

export type GarminAuthResult = GarminAuthChallenge | GarminAuthSuccess;

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
      expiresAt: expiresAt.toISOString(),
      issuedAt: issuedAt.toISOString(),
      tokenLastFour: buildTokenLastFour(email),
    },
  };
}

export function buildInitialSyncSummary(): SyncSummary {
  return {
    lastSyncedAt: new Date().toISOString(),
    rawActivities: 186,
    normalizedActivities: 186,
    status: "ready",
    message:
      "Mock sync completed. Local storage is ready for raw ingestion and normalized analytics.",
  };
}
