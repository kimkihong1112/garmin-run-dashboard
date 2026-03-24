import type { CSSProperties } from "react";

interface EffortRingProps {
  value: number;
  label: string;
  caption: string;
}

export function EffortRing({ value, label, caption }: EffortRingProps) {
  const clampedValue = Math.max(0, Math.min(100, value));

  const style = {
    "--ring-value": `${clampedValue}%`,
  } as CSSProperties;

  return (
    <div className="ring-block">
      <div className="effort-ring" style={style}>
        <div className="effort-ring__inner">
          <span className="effort-ring__value">{clampedValue}%</span>
          <span className="effort-ring__label">{label}</span>
        </div>
      </div>
      <p className="surface-copy">{caption}</p>
    </div>
  );
}
