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

import { useEffect, useRef, useState } from "react";
import { StageChip } from "../components/StageChip";
import type { LogEntry, PipelineStage } from "../lib/types";

export interface ProgressPageProps {
  progress: number;
  currentStage: PipelineStage | null;
  message: string;
  logs: LogEntry[];
  errorMessage: string | null;
  videoTitle: string | null;
  onCancel: () => void;
  onRestart: () => void;
}

const STAGE_ORDER: ReadonlyArray<{ stage: PipelineStage; label: string }> = [
  { stage: "download", label: "下載" },
  { stage: "cropdetect", label: "裁切偵測" },
  { stage: "render", label: "CRT 渲染" },
  { stage: "asr", label: "ASR" },
  { stage: "translate", label: "字幕翻譯" },
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
  videoTitle,
  onCancel,
  onRestart,
}: ProgressPageProps) {
  const logRef = useRef<HTMLDivElement>(null);
  const [now, setNow] = useState(() => Date.now());

  // Live local clock — ticks every second so the user can see wall
  // time and elapsed time without blind waiting (user request 2026-09-14).
  useEffect(() => {
    const id = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(id);
  }, []);

  // Auto-scroll log region to bottom on new lines.
  useEffect(() => {
    const el = logRef.current;
    if (!el) return;
    el.scrollTop = el.scrollHeight;
  }, [logs.length]);

  const pct = Math.max(0, Math.min(1, progress));
  const pctText = `${Math.round(pct * 100)}%`;
  const isError = Boolean(errorMessage);
  const elapsedMs = logs.length > 0 ? now - logs[0].ts : 0;

  return (
    <div className="progress-page">
      <div className="progress-header">
        <h2>{isError ? "轉檔失敗" : "轉檔進行中…"}</h2>
        <span className="progress-pct">{pctText}</span>
      </div>
      {videoTitle ? <div className="progress-video-title">{videoTitle}</div> : null}

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

      <div className="local-clock">
        <span className="local-clock-time">{formatClock(now)}</span>
        <span className="local-clock-sep">·</span>
        <span className="local-clock-elapsed">已等待 {formatElapsed(elapsedMs)}</span>
      </div>

      <div className="progress-actions">
        {isError ? <button className="btn" onClick={onRestart}>返回重試</button> : null}
        <button
          className="btn btn-danger"
          onClick={onCancel}
          disabled={isError}
        >
          取消
        </button>
      </div>

      <div className="neon-copyright" aria-label="(c) copyright 2026 by WUZK">
        <div className="marquee-track" aria-hidden="true">
          <div className="marquee-inner">
            <span className="neon-text">(c) copyright 2026 by WUZK — (c) copyright 2026 by WUZK — (c) copyright 2026 by WUZK — (c) copyright 2026 by WUZK —</span>
            <span className="neon-text" aria-hidden="true">(c) copyright 2026 by WUZK — (c) copyright 2026 by WUZK — (c) copyright 2026 by WUZK — (c) copyright 2026 by WUZK —</span>
          </div>
        </div>
      </div>
    </div>
  );
}

function formatTs(ms: number): string {
  // ms is an epoch millisecond timestamp from Date.now(). Convert to
  // local H:M:S, mod 24h so the displayed time always reads as a
  // wall-clock time the user can recognise.
  // The previous implementation did `m = floor(total / 60)` which
  // produced things like "29821470:20" (minutes-since-epoch wrapped
  // mod 60 seconds) — meaningless to the user. Use a real clock.
  const d = new Date(ms);
  const hh = d.getHours().toString().padStart(2, "0");
  const mm = d.getMinutes().toString().padStart(2, "0");
  const ss = d.getSeconds().toString().padStart(2, "0");
  return `${hh}:${mm}:${ss}`;
}

function formatClock(ms: number): string {
  const d = new Date(ms);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")} ${d.getHours().toString().padStart(2, "0")}:${d.getMinutes().toString().padStart(2, "0")}:${d.getSeconds().toString().padStart(2, "0")}`;
}

function formatElapsed(ms: number): string {
  if (ms <= 0) return "00:00";
  const s = Math.floor(ms / 1000);
  const hh = Math.floor(s / 3600);
  const mm = Math.floor((s % 3600) / 60);
  const ss = s % 60;
  if (hh > 0) return `${String(hh).padStart(2, "0")}:${String(mm).padStart(2, "0")}:${String(ss).padStart(2, "0")}`;
  return `${String(mm).padStart(2, "0")}:${String(ss).padStart(2, "0")}`;
}
