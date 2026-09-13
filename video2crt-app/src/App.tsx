/**
 * App — root of the Video2CRT frontend.
 *
 * Owns the page-state machine (url → options → progress → done) and
 * wires Tauri events into the ProgressPage's state. All long-lived
 * state lives here; pages are presentational.
 *
 * Event lifecycle:
 *   - onReady fires once at startup (currently unused; reserved for
 *     "is api key set" hints).
 *   - onProgress frames update the bar + chips + log region.
 *   - onDone transitions to the DonePage with the final paths.
 *   - onError keeps us on the ProgressPage but flips it into the
 *     "error" visual state and disables Cancel.
 *
 * Cancel: clicking the button invokes `cancel_job(videoId)`. The Rust
 * side sets a flag; the orchestrator exits at the next stage boundary
 * and emits `pipeline://error` with message "cancelled by user". We
 * treat that as a normal error display and don't auto-restart.
 */

import { useCallback, useEffect, useReducer, useRef, useState } from "react";
import { DonePage } from "./pages/DonePage";
import { OptionsPage } from "./pages/OptionsPage";
import { ProgressPage } from "./pages/ProgressPage";
import { UrlPage } from "./pages/UrlPage";
import { SettingsDialog } from "./components/SettingsDialog";
import {
  cancelJob,
  onDone,
  onError,
  onProgress,
  onReady,
  startJob,
} from "./lib/tauri";
import type {
  DoneEvent,
  JobOptions,
  LogEntry,
  PageState,
  PipelineStage,
} from "./lib/types";
import "./App.css";

// Build timestamp injected at vite build time via vite.config.ts's
// `define` option. Shows the user exactly when the running binary was
// compiled — they had complained the "v0.1 · Tauri 2 + React" footer
// never changed, making it impossible to tell if a fix was actually
// deployed.
// Use the declare form so TS doesn't complain about the implicit global.
declare const __VIDEO2CRT_BUILD_TIME__: string;
declare const __VIDEO2CRT_BUILD_EPOCH__: number;

// --------------------------- Constants ---------------------------

const DEFAULT_OPTIONS: JobOptions = {
  // Empty crop string → Rust orchestrator uses cropdetect to pick a
  // per-video crop automatically (gotcha 6 + 24 + 34). Every YouTube video
  // has different pillarbox dimensions, so a hardcoded crop like
  // "960:720:160:0" will eat real content if the source's content area is
  // wider than 960 px (e.g. Louis Armstrong BBC TV was actually 1448 wide).
  crop: "",
  asrLanguage: "auto",
  cloudTranslation: false,
  translationModel: "MiniMax-M3",
};

// --------------------------- State ---------------------------

interface AppState {
  page: PageState;
  url: string;
  options: JobOptions;
  videoId: string | null;
  progress: number;
  currentStage: PipelineStage | null;
  message: string;
  logs: LogEntry[];
  errorMessage: string | null;
  result: DoneEvent | null;
}

type Action =
  | { type: "SET_URL"; url: string }
  | { type: "SET_OPTIONS"; options: JobOptions }
  | { type: "GO"; page: PageState }
  | {
      type: "JOB_STARTED";
      videoId: string;
    }
  | {
      type: "PROGRESS";
      stage: PipelineStage;
      progress: number;
      message: string;
      logLine: string | null;
    }
  | { type: "DONE"; result: DoneEvent }
  | { type: "ERROR"; message: string }
  | { type: "RESET" };

const initialState: AppState = {
  page: "url",
  url: "",
  options: DEFAULT_OPTIONS,
  videoId: null,
  progress: 0,
  currentStage: null,
  message: "",
  logs: [],
  errorMessage: null,
  result: null,
};

function reducer(state: AppState, action: Action): AppState {
  switch (action.type) {
    case "SET_URL":
      return { ...state, url: action.url };
    case "SET_OPTIONS":
      return { ...state, options: action.options };
    case "GO":
      return { ...state, page: action.page };
    case "JOB_STARTED":
      return {
        ...state,
        page: "progress",
        videoId: action.videoId,
        progress: 0,
        currentStage: "init",
        message: "已啟動，等待後端…",
        logs: [],
        errorMessage: null,
        result: null,
      };
    case "PROGRESS": {
      // Always append a log line for each stage transition or
      // status change so the UI's log region scrolls in real time.
      // Previously the reducer only appended when Rust supplied an
      // explicit `logLine`, but stage_begin / stage_done in Rust
      // emit progress with no log_line, so the log region stayed
      // empty even though the pipeline was running. We now record
      // both `logLine` (if present) AND the message text (if it
      // changed since last emit) so the user sees activity.
      const logText =
        action.logLine && action.logLine.length > 0
          ? `[${action.stage}] ${action.logLine}`
          : action.message !== state.message
            ? `[${action.stage}] ${action.message}`
            : null;
      const newLogs =
        logText !== null
          ? [
              ...state.logs,
              {
                ts: Date.now(),
                text: logText,
                stage: action.stage,
              },
            ]
          : state.logs;
      return {
        ...state,
        currentStage: action.stage,
        progress: action.progress,
        message: action.message,
        logs: newLogs,
      };
    }
    case "DONE":
      return {
        ...state,
        page: "done",
        progress: 1,
        currentStage: "done",
        message: "完成",
        result: action.result,
        errorMessage: null,
      };
    case "ERROR":
      return {
        ...state,
        errorMessage: action.message,
      };
    case "RESET":
      return {
        ...initialState,
        url: state.url,
        options: state.options,
      };
  }
}

// --------------------------- Component ---------------------------

export function App() {
  const [state, dispatch] = useReducer(reducer, initialState);
  const [settingsOpen, setSettingsOpen] = useState(false);

  // Refs for the listeners so we can unregister cleanly.
  const stateRef = useRef(state);
  stateRef.current = state;

  // Track job start time so log timestamps stay monotonic across the
  // session — we use Date.now() per line, but expose a consistent epoch.
  const jobStartRef = useRef<number | null>(null);

  // ---- Subscribe to Tauri events once on mount ----
  useEffect(() => {
    const unlistens: Array<Promise<() => void>> = [];

    unlistens.push(
      onReady(() => {
        // Currently a no-op; reserved for future "is api key set" hint.
      }),
    );

    unlistens.push(
      onProgress((p) => {
        if (jobStartRef.current === null) {
          jobStartRef.current = Date.now();
        }
        dispatch({
          type: "PROGRESS",
          stage: p.stage,
          progress: p.progress,
          message: p.message,
          logLine: p.logLine ?? null,
        });
      }),
    );

    unlistens.push(
      onDone((d) => {
        dispatch({ type: "DONE", result: d });
      }),
    );

    unlistens.push(
      onError((e) => {
        dispatch({ type: "ERROR", message: e.message });
      }),
    );

    return () => {
      void Promise.all(unlistens).then((fns) => fns.forEach((fn) => fn()));
    };
  }, []);

  // ---- Handlers ----
  const handleUrlSubmit = useCallback((url: string) => {
    dispatch({ type: "SET_URL", url });
    dispatch({ type: "GO", page: "options" });
  }, []);

  const handleOptionsStart = useCallback(
    async (opts: JobOptions) => {
      dispatch({ type: "SET_OPTIONS", options: opts });
      jobStartRef.current = Date.now();
      try {
        const handle = await startJob({
          url: stateRef.current.url,
          crop: opts.crop,
          asrLanguage: opts.asrLanguage,
          cloudTranslation: opts.cloudTranslation,
          translationModel: opts.cloudTranslation
            ? opts.translationModel
            : null,
        });
        dispatch({ type: "JOB_STARTED", videoId: handle.videoId });
      } catch (e: unknown) {
        dispatch({ type: "ERROR", message: errString(e) });
      }
    },
    [],
  );

  const handleCancel = useCallback(async () => {
    const vid = stateRef.current.videoId;
    if (!vid) return;
    try {
      await cancelJob(vid);
    } catch (e: unknown) {
      dispatch({ type: "ERROR", message: errString(e) });
    }
  }, []);

  const handleRestart = useCallback(() => {
    jobStartRef.current = null;
    dispatch({ type: "RESET" });
  }, []);

  const handleBack = useCallback(() => {
    dispatch({ type: "GO", page: "url" });
  }, []);

  // ---- Render ----
  return (
    <div className="app">
      <header className="app-header">
        <span className="app-title">
          Video2<span className="accent">CRT</span>
        </span>
        <span className="app-step">{stepLabel(state.page)}</span>
      </header>

      <main className="app-body">
        {state.page === "url" ? (
          <UrlPage initialUrl={state.url} onSubmit={handleUrlSubmit} />
        ) : null}

        {state.page === "options" ? (
          <OptionsPage
            url={state.url}
            defaultOptions={state.options}
            onBack={handleBack}
            onOpenSettings={() => setSettingsOpen(true)}
            onStart={(opts) => void handleOptionsStart(opts)}
          />
        ) : null}

        {state.page === "progress" ? (
          <ProgressPage
            progress={state.progress}
            currentStage={state.currentStage}
            message={state.message}
            logs={state.logs}
            errorMessage={state.errorMessage}
            onCancel={() => void handleCancel()}
          />
        ) : null}

        {state.page === "done" && state.result ? (
          <DonePage result={state.result} onRestart={handleRestart} />
        ) : null}
      </main>

      <footer className="app-footer">
        <span>
          v0.1 · build {(__VIDEO2CRT_BUILD_TIME__ || "unknown").slice(0, 19).replace("T", " ")}Z
        </span>
        <span>
          {state.page === "options" || state.page === "url" ? (
            <button
              className="btn btn-ghost"
              onClick={() => setSettingsOpen(true)}
              style={{ padding: "2px 8px", fontSize: 11 }}
            >
              ⚙ 設定
            </button>
          ) : null}
        </span>
      </footer>

      <SettingsDialog
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
      />
    </div>
  );
}

// --------------------------- helpers ---------------------------

function errString(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

function stepLabel(p: PageState): string {
  switch (p) {
    case "url":
      return "1 / 4 · 貼上連結";
    case "options":
      return "2 / 4 · 設定選項";
    case "progress":
      return "3 / 4 · 轉檔中";
    case "done":
      return "4 / 4 · 完成";
  }
}

// LogEntry is exported via re-export so any future helper file can
// import the same type. (Keeps types.ts as the single source of truth.)
export type { LogEntry };
