import { useState, type FormEvent } from "react";
import {
  authenticateGarmin,
  buildInitialSyncSummary,
  MOCK_MFA_HINT,
} from "../../lib/garmin";
import type {
  LoginCredentials,
  LoginSession,
  StorageSnapshot,
  SyncSummary,
} from "../../lib/models";

interface LoginScreenProps {
  bootError: string | null;
  onAuthenticated: (
    session: LoginSession,
    summary: SyncSummary,
  ) => Promise<void>;
  storageSnapshot: StorageSnapshot | null;
}

const INITIAL_VALUES: LoginCredentials = {
  email: "",
  password: "",
  mfaCode: "",
};

export function LoginScreen({
  bootError,
  onAuthenticated,
  storageSnapshot,
}: LoginScreenProps) {
  const [credentials, setCredentials] =
    useState<LoginCredentials>(INITIAL_VALUES);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isMfaVisible, setIsMfaVisible] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const handleChange = (field: keyof LoginCredentials, value: string) => {
    setCredentials((current) => ({
      ...current,
      [field]: value,
    }));
  };

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setError(null);
    setNotice(null);
    setIsSubmitting(true);

    try {
      const result = await authenticateGarmin(credentials);

      if (result.status === "mfa_required") {
        setIsMfaVisible(true);
        setNotice(result.message);
        return;
      }

      await onAuthenticated(result.session, buildInitialSyncSummary());
    } catch (authError) {
      setError(
        authError instanceof Error
          ? authError.message
          : "The Garmin authentication flow could not be completed.",
      );
    } finally {
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
              <span>Current milestone</span>
              <strong>Auth shell, secure storage, and dashboard foundation</strong>
            </div>
          </div>
        </div>
      </section>

      <section className="auth-panel">
        <div className="auth-panel__inner">
          <p className="subtle-kicker">Sign in</p>
          <h2>Connect your Garmin account</h2>
          <p className="surface-copy">
            The current scaffold uses a mocked Garmin adapter while the live
            connector is being implemented.
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
              </label>
            ) : (
              <button
                className="text-button"
                onClick={() => setIsMfaVisible(true)}
                type="button"
              >
                Need to enter a verification code?
              </button>
            )}

            <button className="primary-button" disabled={isSubmitting} type="submit">
              {isSubmitting ? "Authenticating..." : "Sign in securely"}
            </button>
          </form>

          {notice ? <p className="notice-copy">{notice}</p> : null}
          {error ? <p className="error-copy">{error}</p> : null}
          {bootError ? <p className="error-copy">{bootError}</p> : null}

          <div className="auth-footer">
            <p>{MOCK_MFA_HINT}</p>
            <p>
              Tokens are stored in the system keychain in the current macOS
              scaffold.
            </p>
          </div>
        </div>
      </section>
    </main>
  );
}
