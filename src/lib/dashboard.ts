import { invoke } from "@tauri-apps/api/core";
import { DASHBOARD_FALLBACK_SCENARIOS } from "../features/dashboard/dashboard-data";
import type { DashboardRange, DashboardScenario } from "./models";
import { isTauriRuntime } from "./tauri";

export async function loadDashboardScenario(
  range: DashboardRange,
): Promise<DashboardScenario> {
  if (isTauriRuntime()) {
    return invoke<DashboardScenario>("load_dashboard_scenario", { range });
  }

  return DASHBOARD_FALLBACK_SCENARIOS[range];
}
