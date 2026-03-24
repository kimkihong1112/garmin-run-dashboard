export type DashboardRange = "daily" | "weekly" | "monthly";

export interface LoginCredentials {
  email: string;
  password: string;
  mfaCode?: string;
}

export interface LoginSession {
  athleteName: string;
  fullName: string | null;
  accountEmail: string;
  issuedAt: string;
  expiresAt: string;
  tokenLastFour: string;
  unitSystem: string | null;
}

export interface SyncSummary {
  lastSyncedAt: string;
  rawActivities: number;
  normalizedActivities: number;
  status: string;
  message: string;
}

export interface StorageSnapshot {
  appDataDir: string;
  rawDataDir: string;
  normalizedDataDir: string;
  databasePath: string;
  garminAdapterReady: boolean;
  sessionInKeychain: boolean;
  lastSyncSummary: SyncSummary | null;
}

export interface GarminAuthChallenge {
  status: "mfa_required";
  message: string;
}

export interface GarminAuthSuccess {
  status: "authenticated";
  session: LoginSession;
}

export type GarminAuthResult = GarminAuthChallenge | GarminAuthSuccess;

export interface MetricStat {
  label: string;
  value: string;
  delta: string;
}

export interface TrendPoint {
  label: string;
  primary: number;
  accent?: number;
}

export interface DistributionSegment {
  label: string;
  value: number;
  tone: string;
  detail: string;
}

export interface ActivityTableRow {
  title: string;
  date: string;
  distance: string;
  pace: string;
  effort: string;
  values?: string[];
}

export interface HeatmapCell {
  label: string;
  value: number;
  level: number;
}

export interface RoutePoint {
  x: number;
  y: number;
}

export interface GeoRoutePoint {
  latitude: number;
  longitude: number;
}

export interface SplitMetric {
  label: string;
  value: string;
  detail: string;
}

export interface SplitPanel {
  label: string;
  headline: string;
  summary: string;
  time: string;
  pace: string;
  note: string;
  metrics: SplitMetric[];
}

export interface DashboardScenario {
  eyebrow: string;
  title: string;
  description: string;
  isEmpty?: boolean;
  emptyTitle?: string;
  emptyMessage?: string;
  insightTitle: string;
  insight: string;
  keyStats: MetricStat[];
  trendTitle: string;
  trendCaption: string;
  trend: TrendPoint[];
  ring: {
    value: number;
    label: string;
    caption: string;
  };
  distributionTitle: string;
  distribution: DistributionSegment[];
  activityTitle: string;
  activityCaption?: string;
  activityHighlights?: MetricStat[];
  activityColumns?: string[];
  activities: ActivityTableRow[];
  splitPanelsTitle?: string;
  splitPanelsCaption?: string;
  splitPanels?: SplitPanel[];
  notesTitle: string;
  notes: string[];
  routePoints?: RoutePoint[];
  geoRoutePoints?: GeoRoutePoint[];
  heatmapTitle?: string;
  heatmap?: HeatmapCell[];
}
