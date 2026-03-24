import type { DashboardRange, DashboardScenario } from "../../lib/models";

export const DASHBOARD_SEGMENTS = [
  { label: "Daily", value: "daily" },
  { label: "Weekly", value: "weekly" },
  { label: "Monthly", value: "monthly" },
] as const satisfies { label: string; value: DashboardRange }[];

export const DASHBOARD_SCENARIOS: Record<DashboardRange, DashboardScenario> = {
  daily: {
    eyebrow: "Daily review",
    title: "Tuesday tempo execution",
    description:
      "Zoom into one session to understand pacing discipline, heart rate drift, and how cleanly the workout matched the plan.",
    insightTitle: "The tempo stayed smooth until the final two kilometers.",
    insight:
      "Pace stability held inside a narrow range for most of the workout, then heart rate rose faster than speed in the closing block. That suggests the effort cost increased before form fully faded.",
    keyStats: [
      { label: "Distance", value: "12.4 km", delta: "+0.8 km vs planned" },
      { label: "Avg pace", value: "4:36 /km", delta: "6 sec faster than target" },
      { label: "Avg HR", value: "154 bpm", delta: "+4 bpm in final split" },
      { label: "Climb", value: "92 m", delta: "Mostly flat route" },
    ],
    trendTitle: "Pace trend across the workout",
    trendCaption:
      "Primary line shows pace stability. Accent line shows heart-rate drift normalized to the same chart space.",
    trend: [
      { label: "Warm", primary: 4.92, accent: 126 },
      { label: "2 km", primary: 4.58, accent: 141 },
      { label: "4 km", primary: 4.53, accent: 147 },
      { label: "6 km", primary: 4.57, accent: 150 },
      { label: "8 km", primary: 4.54, accent: 153 },
      { label: "10 km", primary: 4.61, accent: 158 },
      { label: "12 km", primary: 4.73, accent: 162 },
    ],
    ring: {
      value: 78,
      label: "Readiness",
      caption:
        "Plenty of quality work landed, but the last segment hints that tomorrow should stay easy to protect the next intensity day.",
    },
    distributionTitle: "Heart-rate zone share",
    distribution: [
      { label: "Z2 aerobic", value: 18, tone: "#ffd6cb", detail: "controlled warmup" },
      { label: "Z3 steady", value: 24, tone: "#ffb59f", detail: "settled into tempo" },
      { label: "Z4 threshold", value: 46, tone: "#ff6a48", detail: "main quality block" },
      { label: "Z5 surge", value: 12, tone: "#cb4026", detail: "closing strain" },
    ],
    activityTitle: "Selected workout splits",
    activities: [
      { title: "Tempo block 1", date: "Mar 24", distance: "3.0 km", pace: "4:35", effort: "Controlled" },
      { title: "Tempo block 2", date: "Mar 24", distance: "3.0 km", pace: "4:34", effort: "Smooth" },
      { title: "Tempo block 3", date: "Mar 24", distance: "3.0 km", pace: "4:36", effort: "Working" },
      { title: "Tempo close", date: "Mar 24", distance: "3.4 km", pace: "4:41", effort: "Tight finish" },
    ],
    notesTitle: "Execution takeaways",
    notes: [
      "Warmup looked disciplined enough to support quality from the first rep.",
      "Cadence and pace stayed efficient until the final quarter of the session.",
      "A short cooldown walk was recorded, but a longer cooldown would improve recovery quality.",
    ],
  },
  weekly: {
    eyebrow: "Weekly review",
    title: "Week of March 23, 2026",
    description:
      "Track weekly volume, workout mix, and how evenly the training load was spread across the week.",
    insightTitle: "Mileage was balanced, but intensity stacked on back-to-back days.",
    insight:
      "The total volume is healthy and the long run stayed under control, yet two moderate-to-hard sessions landed too close together. A slightly calmer Thursday would likely improve weekend sharpness.",
    keyStats: [
      { label: "Distance", value: "68.7 km", delta: "+9% vs previous week" },
      { label: "Time on feet", value: "5h 48m", delta: "+22 min vs previous week" },
      { label: "Runs", value: "6 sessions", delta: "1 rest day" },
      { label: "Long run", value: "22.1 km", delta: "Negative split finish" },
    ],
    trendTitle: "Volume by day",
    trendCaption:
      "Primary line shows distance. Accent line tracks average effort across the same days.",
    trend: [
      { label: "Mon", primary: 8.2, accent: 4.1 },
      { label: "Tue", primary: 12.4, accent: 7.2 },
      { label: "Wed", primary: 9.8, accent: 5.2 },
      { label: "Thu", primary: 11.2, accent: 6.9 },
      { label: "Fri", primary: 0.0, accent: 1.2 },
      { label: "Sat", primary: 5.0, accent: 3.8 },
      { label: "Sun", primary: 22.1, accent: 6.3 },
    ],
    ring: {
      value: 71,
      label: "Weekly balance",
      caption:
        "The workload is productive overall, but redistributing one medium-hard day would create a steadier rhythm through the week.",
    },
    distributionTitle: "Workout mix",
    distribution: [
      { label: "Easy", value: 42, tone: "#ffd6cb", detail: "base-building volume" },
      { label: "Steady", value: 26, tone: "#ffb59f", detail: "moderate aerobic work" },
      { label: "Threshold", value: 20, tone: "#ff6a48", detail: "tempo and cruise reps" },
      { label: "Long run", value: 12, tone: "#cb4026", detail: "extended endurance" },
    ],
    activityTitle: "Key sessions this week",
    activities: [
      { title: "Tuesday tempo", date: "Mar 24", distance: "12.4 km", pace: "4:36", effort: "High quality" },
      { title: "Thursday hills", date: "Mar 26", distance: "11.2 km", pace: "4:58", effort: "Muscular" },
      { title: "Sunday long run", date: "Mar 29", distance: "22.1 km", pace: "5:11", effort: "Controlled" },
      { title: "Saturday shakeout", date: "Mar 28", distance: "5.0 km", pace: "5:34", effort: "Very easy" },
    ],
    notesTitle: "Weekly coaching notes",
    notes: [
      "The week shows strong consistency, especially with only one full rest day.",
      "Thursday's hill session created fatigue that was still visible on Saturday morning.",
      "The long run pace sat in a sustainable range and did not compromise recovery markers.",
    ],
  },
  monthly: {
    eyebrow: "Monthly review",
    title: "March 2026 progression",
    description:
      "Step back and review cumulative volume, consistency, and whether the month created meaningful progression without hidden overload.",
    insightTitle: "The month built cleanly around long-run durability.",
    insight:
      "Long-run frequency stayed reliable, total distance progressed smoothly, and the best efforts improved without a sudden spike in strain. That is the pattern worth repeating into the next block.",
    keyStats: [
      { label: "Distance", value: "212 km", delta: "+14% vs February" },
      { label: "Runs", value: "18 sessions", delta: "4.5 per week" },
      { label: "Longest run", value: "28.0 km", delta: "+3 km monthly peak" },
      { label: "Best 10K", value: "42:18", delta: "Season best" },
    ],
    trendTitle: "Weekly mileage across the month",
    trendCaption:
      "Primary line shows weekly mileage. Accent line tracks perceived freshness across each week.",
    trend: [
      { label: "W1", primary: 44.3, accent: 7.1 },
      { label: "W2", primary: 49.8, accent: 6.8 },
      { label: "W3", primary: 56.2, accent: 6.4 },
      { label: "W4", primary: 61.7, accent: 6.2 },
    ],
    ring: {
      value: 83,
      label: "Block quality",
      caption:
        "The month reads as productive rather than rushed. Volume growth stayed progressive and performance improved without obvious breakdown markers.",
    },
    distributionTitle: "Terrain and effort mix",
    distribution: [
      { label: "Flat aerobic", value: 38, tone: "#ffd6cb", detail: "steady base mileage" },
      { label: "Rolling terrain", value: 22, tone: "#ffb59f", detail: "strength through variety" },
      { label: "Workout quality", value: 24, tone: "#ff6a48", detail: "tempos and intervals" },
      { label: "Long endurance", value: 16, tone: "#cb4026", detail: "weekend durability" },
    ],
    activityTitle: "Monthly signature sessions",
    activities: [
      { title: "Long run peak", date: "Mar 16", distance: "28.0 km", pace: "5:08", effort: "Even and strong" },
      { title: "10K best effort", date: "Mar 19", distance: "10.0 km", pace: "4:14", effort: "Season best" },
      { title: "Cruise intervals", date: "Mar 11", distance: "13.6 km", pace: "4:31", effort: "Locked in" },
      { title: "Recovery week opener", date: "Mar 3", distance: "7.2 km", pace: "5:39", effort: "Reset" },
    ],
    notesTitle: "Month-end takeaways",
    notes: [
      "Long-run rhythm became the strongest repeating signal in the training block.",
      "The best-effort 10K suggests threshold work is converting into race-specific pace.",
      "April should preserve the long-run backbone while adding one carefully measured sharpening session.",
    ],
    heatmapTitle: "Run density calendar",
    heatmap: [
      { label: "Mar 1", value: 6, level: 1 },
      { label: "Mar 2", value: 0, level: 0 },
      { label: "Mar 3", value: 7, level: 2 },
      { label: "Mar 4", value: 11, level: 3 },
      { label: "Mar 5", value: 0, level: 0 },
      { label: "Mar 6", value: 8, level: 1 },
      { label: "Mar 7", value: 18, level: 4 },
      { label: "Mar 8", value: 5, level: 1 },
      { label: "Mar 9", value: 0, level: 0 },
      { label: "Mar 10", value: 9, level: 2 },
      { label: "Mar 11", value: 14, level: 3 },
      { label: "Mar 12", value: 0, level: 0 },
      { label: "Mar 13", value: 10, level: 2 },
      { label: "Mar 14", value: 22, level: 4 },
      { label: "Mar 15", value: 8, level: 1 },
      { label: "Mar 16", value: 28, level: 5 },
      { label: "Mar 17", value: 0, level: 0 },
      { label: "Mar 18", value: 9, level: 2 },
      { label: "Mar 19", value: 10, level: 3 },
      { label: "Mar 20", value: 0, level: 0 },
      { label: "Mar 21", value: 7, level: 1 },
      { label: "Mar 22", value: 16, level: 4 },
      { label: "Mar 23", value: 8, level: 1 },
      { label: "Mar 24", value: 12, level: 3 },
      { label: "Mar 25", value: 10, level: 2 },
      { label: "Mar 26", value: 11, level: 2 },
      { label: "Mar 27", value: 0, level: 0 },
      { label: "Mar 28", value: 5, level: 1 },
      { label: "Mar 29", value: 22, level: 4 },
      { label: "Mar 30", value: 0, level: 0 },
      { label: "Mar 31", value: 8, level: 1 },
      { label: "Apr 1", value: 0, level: 0 },
      { label: "Apr 2", value: 0, level: 0 },
      { label: "Apr 3", value: 0, level: 0 },
      { label: "Apr 4", value: 0, level: 0 },
    ],
  },
};
