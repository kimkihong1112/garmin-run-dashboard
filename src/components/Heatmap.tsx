import type { CSSProperties } from "react";
import type { HeatmapCell } from "../lib/models";

interface HeatmapProps {
  cells: HeatmapCell[];
  title: string;
}

export function Heatmap({ cells, title }: HeatmapProps) {
  return (
    <section className="surface">
      <div className="surface-header">
        <div>
          <p className="surface-kicker">Monthly density</p>
          <h3>{title}</h3>
        </div>
      </div>

      <div className="heatmap-grid" aria-label={title}>
        {cells.map((cell) => (
          <div
            key={cell.label}
            aria-label={`${cell.label}: ${cell.value}`}
            className="heatmap-cell"
            style={
              {
                "--heatmap-level": `${0.18 + cell.level * 0.18}`,
              } as CSSProperties
            }
            title={`${cell.label}: ${cell.value}`}
          />
        ))}
      </div>
    </section>
  );
}
