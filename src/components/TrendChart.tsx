import type { TrendPoint } from "../lib/models";

interface TrendChartProps {
  points: TrendPoint[];
  title: string;
  caption: string;
}

const VIEWBOX_WIDTH = 720;
const VIEWBOX_HEIGHT = 280;
const X_PADDING = 28;
const Y_PADDING = 26;

function buildLinePath(values: number[]) {
  const max = Math.max(...values);
  const min = Math.min(...values);
  const range = Math.max(max - min, 1);

  return values
    .map((value, index) => {
      const x =
        X_PADDING +
        (index * (VIEWBOX_WIDTH - X_PADDING * 2)) / Math.max(values.length - 1, 1);
      const normalized = (value - min) / range;
      const y =
        VIEWBOX_HEIGHT - Y_PADDING - normalized * (VIEWBOX_HEIGHT - Y_PADDING * 2);
      return `${index === 0 ? "M" : "L"} ${x.toFixed(2)} ${y.toFixed(2)}`;
    })
    .join(" ");
}

function buildAreaPath(values: number[]) {
  const linePath = buildLinePath(values);
  const lastX = VIEWBOX_WIDTH - X_PADDING;
  const bottomY = VIEWBOX_HEIGHT - Y_PADDING;

  return `${linePath} L ${lastX} ${bottomY} L ${X_PADDING} ${bottomY} Z`;
}

export function TrendChart({ points, title, caption }: TrendChartProps) {
  const primaryValues = points.map((point) => point.primary);
  const accentValues = points.some((point) => point.accent !== undefined)
    ? points.map((point) => point.accent ?? point.primary)
    : null;

  return (
    <section className="surface surface--chart">
      <div className="surface-header">
        <div>
          <p className="surface-kicker">Trend</p>
          <h3>{title}</h3>
        </div>
        <p className="surface-copy">{caption}</p>
      </div>

      <svg
        className="trend-chart"
        viewBox={`0 0 ${VIEWBOX_WIDTH} ${VIEWBOX_HEIGHT}`}
        role="img"
        aria-label={title}
      >
        <defs>
          <linearGradient id="primaryArea" x1="0" x2="0" y1="0" y2="1">
            <stop offset="0%" stopColor="rgba(255, 106, 72, 0.34)" />
            <stop offset="100%" stopColor="rgba(255, 106, 72, 0.02)" />
          </linearGradient>
        </defs>

        {[0.25, 0.5, 0.75].map((line) => (
          <line
            key={line}
            className="trend-chart__grid"
            x1={X_PADDING}
            x2={VIEWBOX_WIDTH - X_PADDING}
            y1={Y_PADDING + (VIEWBOX_HEIGHT - Y_PADDING * 2) * line}
            y2={Y_PADDING + (VIEWBOX_HEIGHT - Y_PADDING * 2) * line}
          />
        ))}

        <path className="trend-chart__area" d={buildAreaPath(primaryValues)} />

        {accentValues ? (
          <path
            className="trend-chart__line trend-chart__line--accent"
            d={buildLinePath(accentValues)}
          />
        ) : null}

        <path className="trend-chart__line" d={buildLinePath(primaryValues)} />

        {points.map((point, index) => {
          const x =
            X_PADDING +
            (index * (VIEWBOX_WIDTH - X_PADDING * 2)) /
              Math.max(points.length - 1, 1);
          return (
            <text
              key={point.label}
              className="trend-chart__label"
              textAnchor={index === 0 ? "start" : index === points.length - 1 ? "end" : "middle"}
              x={x}
              y={VIEWBOX_HEIGHT - 4}
            >
              {point.label}
            </text>
          );
        })}
      </svg>
    </section>
  );
}
