export type DashboardRange = "daily" | "weekly" | "monthly";

export interface LoginCredentials {
  email: string;
  password: string;
  mfaCode?: string;
}

export interface LoginSession {
  athleteName: string;
  issuedAt: string;
  expiresAt: string;
  tokenLastFour: string;
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
  sessionInKeychain: boolean;
  lastSyncSummary: SyncSummary | null;
}

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
}

export interface HeatmapCell {
  label: string;
  value: number;
  level: number;
}

export interface DashboardScenario {
  eyebrow: string;
  title: string;
  description: string;
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
  activities: ActivityTableRow[];
  notesTitle: string;
  notes: string[];
  heatmapTitle?: string;
  heatmap?: HeatmapCell[];
}
