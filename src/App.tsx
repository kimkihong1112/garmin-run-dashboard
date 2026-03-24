import { useEffect, useState } from "react";
import { LoginScreen } from "./features/auth/LoginScreen";
import { DashboardShell } from "./features/dashboard/DashboardShell";
import { getErrorMessage } from "./lib/errors";
import { buildInitialSyncSummary } from "./lib/garmin";
import type { LoginSession, StorageSnapshot, SyncSummary } from "./lib/models";
import {
  bootstrapLocalStore,
  clearLoginSession,
  clearSyncSummary,
  loadLoginSession,
  loadSyncSummary,
  persistLoginSession,
  persistSyncSummary,
} from "./lib/storage";

function buildDeveloperPreviewSession(): LoginSession {
  const issuedAt = new Date();
  const expiresAt = new Date(issuedAt.getTime() + 1000 * 60 * 60 * 24 * 14);

  return {
    athleteName: "Preview Runner",
    fullName: "Dashboard Preview",
    accountEmail: "preview@local.dev",
    issuedAt: issuedAt.toISOString(),
    expiresAt: expiresAt.toISOString(),
    tokenLastFour: "VIEW",
    unitSystem: "metric",
  };
}

function buildDeveloperPreviewSummary(): SyncSummary {
  return {
    lastSyncedAt: new Date().toISOString(),
    rawActivities: 24,
    normalizedActivities: 24,
    status: "preview",
    message:
      "Developer preview mode is using curated dashboard data so we can iterate without a live Garmin sign-in.",
  };
}

export function App() {
  const [isBooting, setIsBooting] = useState(true);
  const [bootError, setBootError] = useState<string | null>(null);
  const [storageSnapshot, setStorageSnapshot] = useState<StorageSnapshot | null>(
    null,
  );
  const [session, setSession] = useState<LoginSession | null>(null);
  const [syncSummary, setSyncSummary] = useState<SyncSummary>(
    buildInitialSyncSummary(),
  );
  const [isDeveloperPreview, setIsDeveloperPreview] = useState(false);

  useEffect(() => {
    let isMounted = true;

    // App startup prepares local directories, initializes the database, and
    // restores any previously saved session metadata.
    const bootstrap = async () => {
      try {
        const [storage, savedSession, savedSummary] = await Promise.all([
          bootstrapLocalStore(),
          loadLoginSession(),
          loadSyncSummary(),
        ]);

        if (!isMounted) {
          return;
        }

        setStorageSnapshot(storage);
        setSession(savedSession);

        if (savedSummary) {
          setSyncSummary(savedSummary);
        }
      } catch (error) {
        if (!isMounted) {
          return;
        }

        setBootError(
          getErrorMessage(
            error,
            "Failed to prepare the local application storage.",
          ),
        );
      } finally {
        if (isMounted) {
          setIsBooting(false);
        }
      }
    };

    void bootstrap();

    return () => {
      isMounted = false;
    };
  }, []);

  const handleAuthenticated = async (
    nextSession: LoginSession,
    nextSummary: SyncSummary,
  ) => {
    setIsDeveloperPreview(false);
    await persistLoginSession(nextSession);
    await persistSyncSummary(nextSummary);
    setSession(nextSession);
    setSyncSummary(nextSummary);
  };

  const handleOpenDeveloperPreview = () => {
    setBootError(null);
    setIsDeveloperPreview(true);
    setSession(buildDeveloperPreviewSession());
    setSyncSummary(buildDeveloperPreviewSummary());
  };

  const handleSignOut = async () => {
    setIsDeveloperPreview(false);
    await clearLoginSession();
    await clearSyncSummary();
    setSession(null);
    setSyncSummary(buildInitialSyncSummary());
  };

  const handleSyncSummaryChange = async (nextSummary: SyncSummary) => {
    await persistSyncSummary(nextSummary);
    setSyncSummary(nextSummary);
  };

  if (isBooting) {
    return (
      <main className="boot-screen">
        <div className="boot-mark" />
        <p className="subtle-kicker">Preparing local workspace</p>
        <h1>Garmin Run Dashboard</h1>
        <p className="boot-copy">
          Initializing the secure vault, local database, and activity storage
          paths.
        </p>
      </main>
    );
  }

  if (!session) {
    return (
      <LoginScreen
        bootError={bootError}
        currentSyncSummary={syncSummary}
        onAuthenticated={handleAuthenticated}
        onOpenDeveloperPreview={handleOpenDeveloperPreview}
        onSyncSummaryChange={handleSyncSummaryChange}
        storageSnapshot={storageSnapshot}
      />
    );
  }

  return (
    <DashboardShell
      bootError={bootError}
      isPreviewMode={isDeveloperPreview}
      onSignOut={handleSignOut}
      onSyncSummaryChange={handleSyncSummaryChange}
      session={session}
      storageSnapshot={storageSnapshot}
      syncSummary={syncSummary}
    />
  );
}
