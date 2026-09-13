/**
 * ProgressPage — third page of the wizard.
 *
 * State (progress, currentStage, latestMessage, logLines, errorMessage)
 * is owned by App.tsx and passed in as props. This component is purely
 * presentational except for the Cancel button, which invokes the
 * `cancel_job` Rust command.
 *
 * Layout:
 *   1. header with % done
 *   2. 6 chips (init → download → cropdetect → render → asr → burn → mux
 *      → done)  — the "init" stage is implicit at start
 *   3. progress bar
 *   4. current message
 *   5. scrollable log region (tail-end of logLines)
 *   6. Cancel button
 */

import { useEffect, useRef } from "react";
import { StageChip } from "../components/StageChip";
import type { LogEntry, PipelineStage } from "../lib/types";

export interface ProgressPageProps {
  progress: number;
  currentStage: PipelineStage | null;
  message: string;
  logs: LogEntry[];
  errorMessage: string | null;
  onCancel: () => void;
}

const STAGE_ORDER: ReadonlyArray<{ stage: PipelineStage; label: string }> = [
  { stage: "download", label: "下載" },
  { stage: "cropdetect", label: "裁切偵測" },
  { stage: "render", label: "CRT 渲染" },
  { stage: "asr", label: "ASR" },
  { stage: "burn", label: "字幕燒錄" },
  { stage: "mux", label: "封裝" },
];

function stageState(
  stage: PipelineStage,
  current: PipelineStage | null,
  errorMessage: string | null,
): "pending" | "active" | "done" | "error" {
  // "init" is a meta-stage emitted by Rust on job start, before any real
  // stage. Treat it as "no real stage yet" so all chips stay pending until
  // the first real PROGRESS event arrives. Without this guard every chip
  // would render as "done" because stageIndex("init") falls back to
  // Number.MAX_SAFE_INTEGER and any real stage index < MAX. // marker-effectiveCurrent-1789265544.8109665
  const effectiveCurrent = current === "init" ? null : current;
  if (errorMessage) {
    // When something fails, mark everything up to the failing stage done,
    // the current stage as error, and the rest pending.
    if (effectiveCurrent && stageIndex(stage) < stageIndex(effectiveCurrent))
      return "done";
    if (effectiveCurrent && stage === effectiveCurrent) return "error";
    return "pending";
  }
  if (!effectiveCurrent) return "pending";
  if (stage === effectiveCurrent) return "active";
  if (stageIndex(stage) < stageIndex(effectiveCurrent)) return "done";
  return "pending";
}

function stageIndex(s: PipelineStage): number {
  const idx = STAGE_ORDER.findIndex((x) => x.stage === s);
  return idx === -1 ? Number.MAX_SAFE_INTEGER : idx;
}

export function ProgressPage({
  progress,
  currentStage,
  message,
  logs,
  errorMessage,
  onCancel,
}: ProgressPageProps) {
  const logRef = useRef<HTMLDivElement>(null);

  // Auto-scroll log region to bottom on new lines.
  useEffect(() => {
    const el = logRef.current;
    if (!el) return;
    el.scrollTop = el.scrollHeight;
  }, [logs.length]);

  const pct = Math.max(0, Math.min(1, progress));
  const pctText = `${Math.round(pct * 100)}%`;
  const isError = Boolean(errorMessage);

  return (
    <div className="progress-page">
      <div className="progress-header">
        <h2>{isError ? "轉檔失敗" : "轉檔進行中…"}</h2>
        <span className="progress-pct">{pctText}</span>
      </div>

      <div className="stage-chips">
        {STAGE_ORDER.map(({ stage, label }) => (
          <StageChip
            key={stage}
            stage={stage}
            label={label}
            state={stageState(stage, currentStage, errorMessage)}
          />
        ))}
      </div>

      <div className={`progress-bar ${isError ? "error" : ""}`}>
        <div
          className="progress-bar-fill"
          style={{ width: `${pct * 100}%` }}
        />
      </div>

      <div className="progress-message">
        {currentStage ? (
          <span className="stage-tag">{currentStage}</span>
        ) : null}
        {errorMessage ?? message ?? "等待後端…"}
      </div>

      {errorMessage ? (
        <div className="error-banner">{errorMessage}</div>
      ) : null}

      <div className="log-region" ref={logRef}>
        {logs.length === 0 ? (
          <span style={{ color: "#5a6371" }}>（等待日誌輸出…）</span>
        ) : (
          logs.map((line, i) => (
            <span
              key={i}
              className={`log-line stage-${line.stage}`}
            >
              <span className="ts">[{formatTs(line.ts)}]</span>
              {line.text}
            </span>
          ))
        )}
      </div>

      <div className="progress-actions">
        <button
          className="btn btn-danger"
          onClick={onCancel}
          disabled={isError}
        >
          取消
        </button>
      </div>
    </div>
  );
}

function formatTs(ms: number): string {
  const total = Math.floor(ms / 1000);
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
}
