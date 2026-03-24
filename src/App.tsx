import { useEffect, useState } from "react";
import { LoginScreen } from "./features/auth/LoginScreen";
import { DashboardShell } from "./features/dashboard/DashboardShell";
import { buildInitialSyncSummary } from "./lib/garmin";
import type { LoginSession, StorageSnapshot, SyncSummary } from "./lib/models";
import {
  bootstrapLocalStore,
  clearLoginSession,
  loadLoginSession,
  loadSyncSummary,
  persistLoginSession,
  persistSyncSummary,
} from "./lib/storage";

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
          error instanceof Error
            ? error.message
            : "Failed to prepare the local application storage.",
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
    await persistLoginSession(nextSession);
    await persistSyncSummary(nextSummary);
    setSession(nextSession);
    setSyncSummary(nextSummary);
  };

  const handleSignOut = async () => {
    await clearLoginSession();
    setSession(null);
    setSyncSummary(buildInitialSyncSummary());
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
        onAuthenticated={handleAuthenticated}
        storageSnapshot={storageSnapshot}
      />
    );
  }

  return (
    <DashboardShell
      bootError={bootError}
      onSignOut={handleSignOut}
      session={session}
      storageSnapshot={storageSnapshot}
      syncSummary={syncSummary}
    />
  );
}
