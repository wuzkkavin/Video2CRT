/**
 * One progress-stage chip used by ProgressPage.
 *
 * Visual states:
 *   - `pending`: dim, dot gray
 *   - `active`:  accent border + pulsing dot
 *   - `done`:    green border + check style
 *   - `error`:   red border
 */

import type { PipelineStage } from "../lib/types";

export interface StageChipProps {
  stage: PipelineStage;
  label: string;
  state: "pending" | "active" | "done" | "error";
}

export function StageChip({ label, state }: StageChipProps) {
  const className = `chip ${state === "active" ? "active" : ""} ${
    state === "done" ? "done" : ""
  } ${state === "error" ? "error" : ""}`.trim();

  return (
    <span className={className}>
      <span className="dot" />
      <span>{label}</span>
    </span>
  );
}
