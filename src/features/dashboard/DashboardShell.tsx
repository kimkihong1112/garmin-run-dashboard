import { useEffect, useState, type CSSProperties } from "react";
import { EffortRing } from "../../components/EffortRing";
import { Heatmap } from "../../components/Heatmap";
import { RouteGlyph } from "../../components/RouteGlyph";
import { SegmentedControl } from "../../components/SegmentedControl";
import { TrendChart } from "../../components/TrendChart";
import { getErrorMessage } from "../../lib/errors";
import {
  buildSyncErrorSummary,
  buildSyncingSummary,
  syncGarminRunningData,
} from "../../lib/garmin";
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
  isPreviewMode: boolean;
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

const DEFAULT_ACTIVITY_COLUMNS = [
  "Session",
  "Date",
  "Distance",
  "Pace",
  "Effort",
];

function buildActivityGridTemplate(columnCount: number) {
  if (columnCount <= 1) {
    return "minmax(0, 1fr)";
  }

  return `minmax(6.8rem, 1.15fr) repeat(${columnCount - 1}, minmax(6.4rem, 0.92fr))`;
}

function buildActivityValues(
  activity: DashboardScenario["activities"][number],
  columnCount: number,
) {
  const fallbackValues = [
    activity.title,
    activity.date,
    activity.distance,
    activity.pace,
    activity.effort,
  ];
  const baseValues = activity.values ?? fallbackValues;

  if (baseValues.length >= columnCount) {
    return baseValues.slice(0, columnCount);
  }

  return [...baseValues, ...Array.from({ length: columnCount - baseValues.length }, () => "")];
}

export function DashboardShell({
  bootError,
  isPreviewMode,
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
  const [isManualSyncing, setIsManualSyncing] = useState(false);

  useEffect(() => {
    let cancelled = false;

    const loadScenario = async () => {
      setIsLoadingScenario(true);
      setDashboardError(null);

      try {
        if (isPreviewMode) {
          setScenario(DASHBOARD_FALLBACK_SCENARIOS[range]);
          return;
        }

        const nextScenario = await loadDashboardScenario(range);

        if (!cancelled) {
          setScenario(nextScenario);
        }
      } catch (error) {
        if (!cancelled) {
          setDashboardError(
            getErrorMessage(
              error,
              "The dashboard data could not be loaded from local storage.",
            ),
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
  }, [isPreviewMode, range, syncSummary.lastSyncedAt]);

  const handleSyncNow = async () => {
    setDashboardError(null);
    setIsManualSyncing(true);

    if (isPreviewMode) {
      await onSyncSummaryChange({
        lastSyncedAt: new Date().toISOString(),
        rawActivities: 24,
        normalizedActivities: 24,
        status: "preview",
        message:
          "Developer preview refreshed the mock dashboard state without calling Garmin.",
      });
      setIsManualSyncing(false);
      return;
    }

    await onSyncSummaryChange(
      buildSyncingSummary(
        syncSummary,
        "Refreshing your local Garmin running data from the desktop adapter.",
      ),
    );

    try {
      const nextSummary = await syncGarminRunningData();
      await onSyncSummaryChange(nextSummary);
    } catch (error) {
      await onSyncSummaryChange(
        buildSyncErrorSummary(
          syncSummary,
          getErrorMessage(
            error,
            "The Garmin sync did not complete successfully.",
          ),
        ),
      );
      setDashboardError(
        getErrorMessage(
          error,
          "The Garmin sync did not complete successfully.",
        ),
      );
    } finally {
      setIsManualSyncing(false);
    }
  };

  const lastSyncLabel =
    syncSummary.rawActivities > 0 ? formatTimestamp(syncSummary.lastSyncedAt) : "Not yet";
  const activityColumns = scenario.activityColumns ?? DEFAULT_ACTIVITY_COLUMNS;
  const activityTableStyle = {
    "--activity-grid-template": buildActivityGridTemplate(activityColumns.length),
  } as CSSProperties;
  const isSyncing = isManualSyncing || syncSummary.status === "syncing";
  const recentActivitySection = (
    <section className="surface session-hero__activity">
      <div className="surface-header">
        <div>
          <p className="surface-kicker">Recent activity</p>
          <h3>{scenario.activityTitle}</h3>
        </div>
      </div>

      {scenario.activityCaption ? (
        <p className="surface-copy session-hero__activity-copy">{scenario.activityCaption}</p>
      ) : null}

      {scenario.activityHighlights?.length ? (
        <div className="activity-highlight-strip">
          {scenario.activityHighlights.map((highlight) => (
            <article className="activity-highlight" key={highlight.label}>
              <span>{highlight.label}</span>
              <strong>{highlight.value}</strong>
              <small>{highlight.delta}</small>
            </article>
          ))}
        </div>
      ) : null}

      <div
        className="activity-table"
        role="table"
        aria-label={scenario.activityTitle}
        style={activityTableStyle}
      >
        <div className="activity-table__header" role="row">
          {activityColumns.map((column) => (
            <span key={column}>{column}</span>
          ))}
        </div>

        {scenario.activities.map((activity) => (
          <div className="activity-table__row" key={activity.title} role="row">
            {buildActivityValues(activity, activityColumns.length).map((value, index) => (
              <span key={`${activity.title}-${activityColumns[index]}`}>{value}</span>
            ))}
          </div>
        ))}
      </div>
    </section>
  );

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
          <strong>
            {isPreviewMode
              ? "Developer preview"
              : storageSnapshot
                ? "Keychain backed"
                : "Browser preview mode"}
          </strong>
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
          <p>
            {isPreviewMode
              ? "Using curated mock analytics so we can iterate on the dashboard layout quickly."
              : storageSnapshot?.databasePath ?? "Prepared when running inside Tauri."}
          </p>
        </div>

        <div className="sidebar-actions">
          <button
            className="primary-button"
            disabled={isSyncing}
            onClick={() => void handleSyncNow()}
            type="button"
          >
            {isPreviewMode
              ? isSyncing
                ? "Refreshing preview..."
                : "Refresh preview"
              : isSyncing
                ? "Syncing Garmin..."
                : "Sync now"}
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
            <section className="session-hero">
              <div className="session-hero__story">
                <p className="surface-kicker">Session read</p>
                <h2>{scenario.insightTitle}</h2>
                <p className="workspace-copy">{scenario.insight}</p>
                <div className="session-story__facts">
                  <span>{scenario.activities.length} visible splits</span>
                  <span>{scenario.routePoints ? "GPS trace included" : "Summary-only view"}</span>
                  <span>{isPreviewMode ? "Preview data" : "Live local analytics"}</span>
                </div>
              </div>

              <div className="session-hero__route">
                <div className="session-hero__route-header">
                  <div>
                    <p className="surface-kicker">
                      {scenario.routePoints ? "Route trace" : "Primary context"}
                    </p>
                    <h3>{scenario.title}</h3>
                  </div>
                  {isPreviewMode ? <span className="preview-pill">FIT-backed preview</span> : null}
                </div>

                {scenario.routePoints ? (
                  <RouteGlyph
                    geoPoints={scenario.geoRoutePoints}
                    points={scenario.routePoints}
                    title={scenario.title}
                  />
                ) : (
                  <div className="route-placeholder" aria-hidden="true" />
                )}

                <p className="surface-copy">{scenario.description}</p>
              </div>

              <div className="metric-strip metric-strip--rail">
                {scenario.keyStats.map((stat) => (
                  <article className="metric-tile metric-tile--rail" key={stat.label}>
                    <span>{stat.label}</span>
                    <div className="metric-tile__value-row">
                      <strong>{stat.value}</strong>
                      <small>{stat.delta}</small>
                    </div>
                  </article>
                ))}
              </div>

              {recentActivitySection}
            </section>

            <section className="dashboard-grid">
              <div className="dashboard-grid__main">
                <TrendChart
                  caption={scenario.trendCaption}
                  points={scenario.trend}
                  title={scenario.trendTitle}
                />

                {scenario.splitPanels?.length ? (
                  <section className="surface split-inspector">
                    <div className="surface-header split-inspector__header">
                      <div>
                        <p className="surface-kicker">Raw lap metrics</p>
                        <h3>{scenario.splitPanelsTitle ?? "Split inspector"}</h3>
                      </div>
                      {scenario.splitPanelsCaption ? (
                        <p className="surface-copy split-inspector__copy">
                          {scenario.splitPanelsCaption}
                        </p>
                      ) : null}
                    </div>

                    <div className="split-panel-list">
                      {scenario.splitPanels.map((split) => (
                        <article className="split-panel" key={split.label}>
                          <div className="split-panel__top">
                            <div className="split-panel__copy">
                              <span className="split-panel__label">{split.label}</span>
                              <h4>{split.headline}</h4>
                              <p>{split.summary}</p>
                            </div>

                            <div className="split-panel__timing">
                              <strong>{split.time}</strong>
                              <span>{split.pace}</span>
                            </div>
                          </div>

                          <div className="split-panel__metrics">
                            {split.metrics.map((metric) => (
                              <div className="split-metric" key={`${split.label}-${metric.label}`}>
                                <span>{metric.label}</span>
                                <strong>{metric.value}</strong>
                                <small>{metric.detail}</small>
                              </div>
                            ))}
                          </div>

                          <p className="split-panel__note">{split.note}</p>
                        </article>
                      ))}
                    </div>
                  </section>
                ) : null}
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
