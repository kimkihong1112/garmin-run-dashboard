import { useEffect, useState } from "react";
import { EffortRing } from "../../components/EffortRing";
import { Heatmap } from "../../components/Heatmap";
import { SegmentedControl } from "../../components/SegmentedControl";
import { TrendChart } from "../../components/TrendChart";
import { syncGarminRunningData } from "../../lib/garmin";
import { loadDashboardScenario } from "../../lib/dashboard";
import type {
  DashboardRange,
  DashboardScenario,
  DistributionSegment,
  LoginSession,
  StorageSnapshot,
  SyncSummary,
} from "../../lib/models";
import { DASHBOARD_FALLBACK_SCENARIOS, DASHBOARD_SEGMENTS } from "./dashboard-data";

interface DashboardShellProps {
  bootError: string | null;
  onSignOut: () => Promise<void>;
  onSyncSummaryChange: (summary: SyncSummary) => Promise<void>;
  session: LoginSession;
  storageSnapshot: StorageSnapshot | null;
  syncSummary: SyncSummary;
}

function formatTimestamp(value: string) {
  const date = new Date(value);

  return new Intl.DateTimeFormat("en-US", {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }).format(date);
}

function DistributionList({ items }: { items: DistributionSegment[] }) {
  return (
    <div className="distribution-list">
      {items.map((item) => (
        <div className="distribution-row" key={item.label}>
          <div className="distribution-row__copy">
            <strong>{item.label}</strong>
            <span>{item.detail}</span>
          </div>
          <div className="distribution-row__track">
            <div
              className="distribution-row__fill"
              style={{
                width: `${item.value}%`,
                background: item.tone,
              }}
            />
          </div>
          <span className="distribution-row__value">{item.value}%</span>
        </div>
      ))}
    </div>
  );
}

export function DashboardShell({
  bootError,
  onSignOut,
  onSyncSummaryChange,
  session,
  storageSnapshot,
  syncSummary,
}: DashboardShellProps) {
  const [range, setRange] = useState<DashboardRange>("daily");
  const [scenario, setScenario] = useState<DashboardScenario>(
    DASHBOARD_FALLBACK_SCENARIOS.daily,
  );
  const [isLoadingScenario, setIsLoadingScenario] = useState(true);
  const [dashboardError, setDashboardError] = useState<string | null>(null);
  const [isSyncing, setIsSyncing] = useState(false);

  useEffect(() => {
    let cancelled = false;

    const loadScenario = async () => {
      setIsLoadingScenario(true);
      setDashboardError(null);

      try {
        const nextScenario = await loadDashboardScenario(range);

        if (!cancelled) {
          setScenario(nextScenario);
        }
      } catch (error) {
        if (!cancelled) {
          setDashboardError(
            error instanceof Error
              ? error.message
              : "The dashboard data could not be loaded from local storage.",
          );
          setScenario({
            ...DASHBOARD_FALLBACK_SCENARIOS[range],
            isEmpty: true,
            emptyTitle: "The dashboard could not be loaded from the local database.",
            emptyMessage:
              "Try syncing Garmin again. If the issue continues, inspect the local SQLite and raw activity store.",
          });
        }
      } finally {
        if (!cancelled) {
          setIsLoadingScenario(false);
        }
      }
    };

    void loadScenario();

    return () => {
      cancelled = true;
    };
  }, [range, syncSummary.lastSyncedAt]);

  const handleSyncNow = async () => {
    setDashboardError(null);
    setIsSyncing(true);

    try {
      const nextSummary = await syncGarminRunningData();
      await onSyncSummaryChange(nextSummary);
    } catch (error) {
      setDashboardError(
        error instanceof Error
          ? error.message
          : "The Garmin sync did not complete successfully.",
      );
    } finally {
      setIsSyncing(false);
    }
  };

  const lastSyncLabel =
    syncSummary.rawActivities > 0 ? formatTimestamp(syncSummary.lastSyncedAt) : "Not yet";

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand-lockup">
          <div className="brand-mark" />
          <div>
            <p className="subtle-kicker">Garmin Run Dashboard</p>
            <strong>{session.athleteName}</strong>
          </div>
        </div>

        <SegmentedControl
          onChange={setRange}
          options={DASHBOARD_SEGMENTS}
          value={range}
        />

        <div className="sidebar-block">
          <span>Session vault</span>
          <strong>{storageSnapshot ? "Keychain backed" : "Browser preview mode"}</strong>
        </div>

        <div className="sidebar-block">
          <span>Last sync</span>
          <strong>{lastSyncLabel}</strong>
        </div>

        <div className="sidebar-block">
          <span>Stored activities</span>
          <strong>
            {syncSummary.normalizedActivities} normalized / {syncSummary.rawActivities} raw
          </strong>
        </div>

        <div className="sidebar-block sidebar-block--muted">
          <span>Database</span>
          <p>{storageSnapshot?.databasePath ?? "Prepared when running inside Tauri."}</p>
        </div>

        <div className="sidebar-actions">
          <button
            className="primary-button"
            disabled={isSyncing}
            onClick={() => void handleSyncNow()}
            type="button"
          >
            {isSyncing ? "Syncing Garmin..." : "Sync now"}
          </button>

          <button className="secondary-button" onClick={() => void onSignOut()} type="button">
            Sign out
          </button>
        </div>

      </aside>

      <main className="workspace">
        <header className="workspace-header">
          <div>
            <p className="subtle-kicker">{scenario.eyebrow}</p>
            <h1>{scenario.title}</h1>
            <p className="workspace-copy">{scenario.description}</p>
          </div>

          <div className="header-meta">
            <div className="status-chip">
              <span className="status-chip__dot" />
              {syncSummary.status}
            </div>
            <p>{syncSummary.message}</p>
          </div>
        </header>

        {bootError ? <p className="error-copy">{bootError}</p> : null}
        {dashboardError ? <p className="error-copy">{dashboardError}</p> : null}

        {isLoadingScenario ? (
          <section className="surface empty-state">
            <p className="surface-kicker">Loading analytics</p>
            <h3>Preparing the {range} dashboard from local SQLite data.</h3>
            <p className="workspace-copy">
              The app is reading normalized Garmin activities and calculating the
              latest summaries.
            </p>
          </section>
        ) : scenario.isEmpty ? (
          <section className="surface empty-state">
            <p className="surface-kicker">No running data yet</p>
            <h3>{scenario.emptyTitle ?? "Import Garmin activities to unlock the dashboard."}</h3>
            <p className="workspace-copy">
              {scenario.emptyMessage ??
                "Your local database does not contain enough running data for this view yet."}
            </p>
            <button
              className="primary-button empty-state__button"
              disabled={isSyncing}
              onClick={() => void handleSyncNow()}
              type="button"
            >
              {isSyncing ? "Syncing Garmin..." : "Run the first sync"}
            </button>
          </section>
        ) : (
          <>
            <section className="hero-surface">
              <div className="hero-surface__copy">
                <p className="surface-kicker">Primary insight</p>
                <h2>{scenario.insightTitle}</h2>
                <p className="workspace-copy">{scenario.insight}</p>
              </div>

              <div className="metric-strip">
                {scenario.keyStats.map((stat) => (
                  <div className="metric-tile" key={stat.label}>
                    <span>{stat.label}</span>
                    <strong>{stat.value}</strong>
                    <small>{stat.delta}</small>
                  </div>
                ))}
              </div>
            </section>

            <section className="dashboard-grid">
              <div className="dashboard-grid__main">
                <TrendChart
                  caption={scenario.trendCaption}
                  points={scenario.trend}
                  title={scenario.trendTitle}
                />

                <section className="surface">
                  <div className="surface-header">
                    <div>
                      <p className="surface-kicker">Recent activity</p>
                      <h3>{scenario.activityTitle}</h3>
                    </div>
                  </div>

                  <div
                    className="activity-table"
                    role="table"
                    aria-label={scenario.activityTitle}
                  >
                    <div className="activity-table__header" role="row">
                      <span>Session</span>
                      <span>Date</span>
                      <span>Distance</span>
                      <span>Pace</span>
                      <span>Effort</span>
                    </div>

                    {scenario.activities.map((activity) => (
                      <div className="activity-table__row" key={activity.title} role="row">
                        <span>{activity.title}</span>
                        <span>{activity.date}</span>
                        <span>{activity.distance}</span>
                        <span>{activity.pace}</span>
                        <span>{activity.effort}</span>
                      </div>
                    ))}
                  </div>
                </section>
              </div>

              <div className="dashboard-grid__side">
                <section className="surface">
                  <div className="surface-header">
                    <div>
                      <p className="surface-kicker">Training state</p>
                      <h3>{scenario.ring.label}</h3>
                    </div>
                  </div>

                  <EffortRing
                    caption={scenario.ring.caption}
                    label={scenario.ring.label}
                    value={scenario.ring.value}
                  />
                </section>

                <section className="surface">
                  <div className="surface-header">
                    <div>
                      <p className="surface-kicker">Distribution</p>
                      <h3>{scenario.distributionTitle}</h3>
                    </div>
                  </div>

                  <DistributionList items={scenario.distribution} />
                </section>

                <section className="surface">
                  <div className="surface-header">
                    <div>
                      <p className="surface-kicker">Coach notes</p>
                      <h3>{scenario.notesTitle}</h3>
                    </div>
                  </div>

                  <ul className="note-list">
                    {scenario.notes.map((note) => (
                      <li key={note}>{note}</li>
                    ))}
                  </ul>
                </section>

                {scenario.heatmap ? (
                  <Heatmap
                    cells={scenario.heatmap}
                    title={scenario.heatmapTitle ?? "Run density calendar"}
                  />
                ) : null}
              </div>
            </section>
          </>
        )}
      </main>
    </div>
  );
}
