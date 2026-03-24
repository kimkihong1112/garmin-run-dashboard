import { useEffect, useState, type FormEvent } from "react";
import { getErrorMessage } from "../../lib/errors";
import {
  authenticateGarmin,
  buildSyncErrorSummary,
  buildSyncingSummary,
  MOCK_MFA_HINT,
  syncGarminRunningData,
} from "../../lib/garmin";
import type {
  GarminAuthResult,
  LoginCredentials,
  LoginSession,
  StorageSnapshot,
  SyncSummary,
} from "../../lib/models";

interface LoginScreenProps {
  bootError: string | null;
  currentSyncSummary: SyncSummary;
  onAuthenticated: (
    session: LoginSession,
    summary: SyncSummary,
  ) => Promise<void>;
  onOpenDeveloperPreview: () => void;
  onSyncSummaryChange: (summary: SyncSummary) => Promise<void>;
  storageSnapshot: StorageSnapshot | null;
}

const INITIAL_VALUES: LoginCredentials = {
  email: "",
  password: "",
  mfaCode: "",
};

export function LoginScreen({
  bootError,
  currentSyncSummary,
  onAuthenticated,
  onOpenDeveloperPreview,
  onSyncSummaryChange,
  storageSnapshot,
}: LoginScreenProps) {
  const [credentials, setCredentials] =
    useState<LoginCredentials>(INITIAL_VALUES);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isMfaVisible, setIsMfaVisible] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [progressHint, setProgressHint] = useState<string | null>(null);
  const [submitPhase, setSubmitPhase] = useState<"idle" | "auth" | "sync">("idle");

  useEffect(() => {
    if (!isSubmitting) {
      setProgressHint(null);
      return;
    }

    const timer = window.setTimeout(() => {
      setProgressHint(
        submitPhase === "auth"
          ? "Still waiting on Garmin Connect. This can take a few seconds, especially when Garmin is preparing an email verification challenge."
          : "The app has your session and is still importing recent runs in the background.",
      );
    }, 5000);

    return () => {
      window.clearTimeout(timer);
    };
  }, [isSubmitting, submitPhase]);

  const handleChange = (field: keyof LoginCredentials, value: string) => {
    setCredentials((current) => ({
      ...current,
      [field]: value,
    }));
  };

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setError(null);
    setNotice("Connecting to Garmin Connect...");
    setProgressHint(null);
    setIsSubmitting(true);
    setSubmitPhase("auth");

    try {
      const result: GarminAuthResult = await authenticateGarmin(credentials);

      if (result.status === "mfa_required") {
        setIsMfaVisible(true);
        setNotice(
          result.message ||
            "Garmin requested a verification code. Check your email and enter the code below.",
        );
        setSubmitPhase("idle");
        return;
      }

      setNotice("Authenticated. Opening your dashboard...");
      setSubmitPhase("sync");

      const pendingSummary = buildSyncingSummary(currentSyncSummary);
      await onAuthenticated(result.session, pendingSummary);

      const runBackgroundSync = async () => {
        try {
          const syncSummary = await syncGarminRunningData();
          await onSyncSummaryChange(syncSummary);
        } catch (syncError) {
          await onSyncSummaryChange(
            buildSyncErrorSummary(
              currentSyncSummary,
              getErrorMessage(
                syncError,
                "Garmin sign-in succeeded, but the first sync did not complete.",
              ),
            ),
          );
        }
      };

      void runBackgroundSync();
    } catch (authError) {
      setNotice(null);
      setError(
        getErrorMessage(
          authError,
          "The Garmin authentication flow could not be completed.",
        ),
      );
    } finally {
      setSubmitPhase("idle");
      setIsSubmitting(false);
    }
  };

  return (
    <main className="auth-shell">
      <section className="auth-visual">
        <div className="auth-orb auth-orb--primary" />
        <div className="auth-orb auth-orb--secondary" />
        <div className="auth-orb auth-orb--tertiary" />

        <div className="auth-visual__content">
          <p className="subtle-kicker">Local-first desktop analytics</p>
          <h1>Understand each run with less noise and more signal.</h1>
          <p className="auth-copy">
            Import Garmin Connect history to your Mac, keep the raw payloads on
            disk, and review daily, weekly, and monthly performance without
            sending your private training data to another cloud.
          </p>

          <div className="auth-bullets">
            <div>
              <span>Secure session vault</span>
              <strong>
                {storageSnapshot ? "macOS Keychain ready" : "Prepared in preview mode"}
              </strong>
            </div>
            <div>
              <span>Data residency</span>
              <strong>SQLite + raw JSON stored locally</strong>
            </div>
            <div>
              <span>Garmin adapter</span>
              <strong>
                {storageSnapshot?.garminAdapterReady
                  ? "Python adapter configured"
                  : "Setup required or browser preview mode"}
              </strong>
            </div>
          </div>
        </div>
      </section>

      <section className="auth-panel">
        <div className="auth-panel__inner">
          <p className="subtle-kicker">Sign in</p>
          <h2>Connect your Garmin account</h2>
          <p className="surface-copy">
            The desktop runtime signs in through a local Python adapter, then
            saves the Garmin token payload in the system keychain.
          </p>

          <form className="auth-form" onSubmit={handleSubmit}>
            <label className="field">
              <span>Garmin email</span>
              <input
                autoComplete="username"
                name="email"
                onChange={(event) => handleChange("email", event.target.value)}
                placeholder="runner@example.com"
                type="email"
                value={credentials.email}
              />
            </label>

            <label className="field">
              <span>Password</span>
              <input
                autoComplete="current-password"
                name="password"
                onChange={(event) => handleChange("password", event.target.value)}
                placeholder="Enter your Garmin password"
                type="password"
                value={credentials.password}
              />
            </label>

            {isMfaVisible ? (
              <label className="field field--mfa">
                <span>Verification code</span>
                <input
                  inputMode="numeric"
                  maxLength={6}
                  name="mfaCode"
                  onChange={(event) => handleChange("mfaCode", event.target.value)}
                  placeholder="6-digit code"
                  type="text"
                  value={credentials.mfaCode}
                />
                <small className="field-help">
                  Garmin requested MFA. Enter the verification code sent to your
                  email to continue.
                </small>
              </label>
            ) : null}

            <button className="primary-button" disabled={isSubmitting} type="submit">
              {isSubmitting
                ? submitPhase === "sync"
                  ? "Opening dashboard..."
                  : "Connecting to Garmin..."
                : "Sign in securely"}
            </button>

            <button
              className="secondary-button auth-preview-button"
              onClick={onOpenDeveloperPreview}
              type="button"
            >
              Open dashboard preview
            </button>
          </form>

          {isSubmitting ? (
            <div aria-live="polite" className="auth-progress" role="status">
              <span aria-hidden="true" className="auth-progress__spinner" />
              <div>
                <strong>
                  {submitPhase === "sync"
                    ? "Preparing your dashboard"
                    : "Contacting Garmin Connect"}
                </strong>
                <p>
                  {submitPhase === "sync"
                    ? "Sign-in completed. Recent runs are being imported in the background so the app can move forward immediately."
                    : "Checking your credentials and waiting to see whether Garmin asks for an email verification code."}
                </p>
              </div>
            </div>
          ) : null}

          {progressHint ? <p className="notice-copy">{progressHint}</p> : null}
          {notice && !progressHint ? <p className="notice-copy">{notice}</p> : null}
          {error ? <p className="error-copy">{error}</p> : null}
          {bootError ? <p className="error-copy">{bootError}</p> : null}

          <div className="auth-footer">
            <p>{MOCK_MFA_HINT}</p>
            <p>
              Tokens are stored in the system keychain in the current macOS
              scaffold.
            </p>
            <p>
              The verification code field appears only after Garmin confirms
              that MFA is required for the current sign-in attempt.
            </p>
            <p>
              Developer preview opens the dashboard with curated local mock
              data and does not require a Garmin login.
            </p>
          </div>
        </div>
      </section>
    </main>
  );
}
