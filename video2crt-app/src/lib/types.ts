/**
 * Shared TypeScript types mirroring the Rust serde structs in
 * `src-tauri/src/`. All field names are camelCase to match the
 * `#[serde(rename_all = "camelCase")]` attribute on the Rust side.
 *
 * Keep this file in sync with the Rust models whenever the wire format
 * changes. The point of having a single source of truth here is to catch
 * type drift at compile time on the React side.
 */

// ---------- Tauri commands ----------

/** Payload sent to the `start_job` Rust command. */
export interface StartJobRequest {
  url: string;
  /** Optional override of the project root. */
  projectRoot?: string | null;
  /**
   * Crop value, e.g. "960:720:160:0" (gotcha 6 + 24). Leave empty/null to let
   * the Rust orchestrator run `ffmpeg cropdetect` and pick a per-video
   * crop automatically. This is the recommended default — every YouTube
   * video has different pillarbox dimensions, so a hardcoded crop will
   * eat real content on any source wider than the crop width.
   */
  crop?: string | null;
  /** ASR language hint: "ja" | "en" | "zh" | "auto". */
  asrLanguage?: string | null;
  /** Whether to generate subtitles at all (default true). If false, ASR/SRT/burn are skipped and raw.mp4 is muxed directly. */
  enableSubtitles?: boolean | null;
  /** Use cloud translation via Minimax (default false). Only meaningful when enableSubtitles=true. */
  cloudTranslation: boolean;
  /** Model id for cloud translation (required when cloudTranslation=true). */
  translationModel?: string | null;
  /**
   * Absolute output directory picked by the user in OptionsPage.
   * Empty/null → Rust falls back to `<projectRoot>/output/yt_<id>/`.
   */
  outputDir?: string | null;
}

/** Result of a successful `start_job` invocation. */
export interface JobHandle {
  videoId: string;
  outputDir: string;
}

// ---------- Tauri events ----------

/** All pipeline stage identifiers emitted by the orchestrator. */
export type PipelineStage =
  | "init"
  | "download"
  | "cropdetect"
  | "render"
  | "asr"
  | "burn"
  | "mux"
  | "done";

/** Single progress frame delivered on the `pipeline://progress` channel. */
export interface ProgressEvent {
  videoId: string;
  stage: PipelineStage;
  /** Overall job progress, 0.0..=1.0. */
  progress: number;
  message: string;
  /** Optional incremental log line forwarded from the worker. */
  logLine?: string | null;
}

/** Payload of the `pipeline://done` event. */
export interface DoneEvent {
  videoId: string;
  outputDir: string;
  finalMp4: string;
  srt: string;
}

/** Payload of the `pipeline://error` event. */
export interface ErrorEvent {
  videoId: string;
  message: string;
}

/** Payload of the `pipeline://ready` event (currently empty object). */
export type ReadyEvent = Record<string, never>;

// ---------- Translator ----------

/** Mirrors `translator::ModelInfo`. */
export interface ModelInfo {
  id: string;
  label: string;
  /** "language" | "video" | "speech" | "other" */
  group: string;
  isDefault: boolean;
}

// ---------- UI state ----------

/** The four top-level page states the App cycles through. */
export type PageState = "url" | "options" | "progress" | "done";

/** A single log line kept in memory for the ProgressPage log region. */
export interface LogEntry {
  /** Monotonic timestamp in ms since the job started. */
  ts: number;
  text: string;
  /** Stage at the time the line was captured (for color coding). */
  stage: PipelineStage;
}

/** Options captured on the OptionsPage, ready to be submitted. */
export interface JobOptions {
  crop: string;
  asrLanguage: "auto" | "ja" | "en" | "zh";
  enableSubtitles: boolean;
  cloudTranslation: boolean;
  translationModel: string;
  /**
   * Directory to write `source.mp4 / raw.mp4 / subtitled.mp4 /
   * final.mp4 / *.srt` into. Defaults to `<projectRoot>/output/yt_<id>`
   * if empty. The user can override this from OptionsPage via a
   * folder picker; we keep the path absolute on the Rust side.
   */
  outputDir: string;
}
