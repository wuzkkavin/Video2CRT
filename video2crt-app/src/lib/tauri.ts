/**
 * Typed wrappers around Tauri's `invoke` and `listen` APIs.
 *
 * The whole point of this module is to centralise the (cmd, args) ↔ TS
 * type pairing so the rest of the app can call `startJob(...)` instead
 * of `invoke<JobHandle>("start_job", { req })`. Drift between Rust
 * commands and React call sites becomes a compile error instead of a
 * runtime mystery.
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  DoneEvent,
  ErrorEvent,
  JobHandle,
  ModelInfo,
  ProgressEvent,
  ReadyEvent,
  StartJobRequest,
} from "./types";

// ---------- Commands ----------

/** Kick off the CRT conversion pipeline. Resolves with a JobHandle. */
export function startJob(req: StartJobRequest): Promise<JobHandle> {
  return invoke<JobHandle>("start_job", { req });
}

/** Request cancellation of a running job by video_id. */
export function cancelJob(videoId: string): Promise<void> {
  return invoke<void>("cancel_job", { videoId });
}

/** Open the system file manager and select the given path. */
export function revealInExplorer(path: string): Promise<void> {
  return invoke<void>("reveal_in_explorer", { path });
}

/** Check whether a cloud translation API key is currently stored. */
export function hasApiKey(): Promise<boolean> {
  return invoke<boolean>("has_api_key");
}

/** Persist a cloud translation API key in Windows Credential Manager. */
export function saveApiKey(key: string): Promise<void> {
  return invoke<void>("save_api_key", { key });
}

/** Remove the stored cloud translation API key. */
export function deleteApiKey(): Promise<void> {
  return invoke<void>("delete_api_key");
}

/** List available cloud translation models (live or hard-coded fallback). */
export function listTranslationModels(): Promise<ModelInfo[]> {
  return invoke<ModelInfo[]>("list_translation_models");
}

// ---------- Events ----------

/** Subscribe to `pipeline://ready` (fires once at app startup). */
export function onReady(
  cb: (payload: ReadyEvent) => void,
): Promise<UnlistenFn> {
  return listen<ReadyEvent>("pipeline://ready", (e) => cb(e.payload));
}

/** Subscribe to `pipeline://progress`. */
export function onProgress(
  cb: (payload: ProgressEvent) => void,
): Promise<UnlistenFn> {
  return listen<ProgressEvent>("pipeline://progress", (e) => cb(e.payload));
}

/** Subscribe to `pipeline://done`. */
export function onDone(
  cb: (payload: DoneEvent) => void,
): Promise<UnlistenFn> {
  return listen<DoneEvent>("pipeline://done", (e) => cb(e.payload));
}

/** Subscribe to `pipeline://error`. */
export function onError(
  cb: (payload: ErrorEvent) => void,
): Promise<UnlistenFn> {
  return listen<ErrorEvent>("pipeline://error", (e) => cb(e.payload));
}
