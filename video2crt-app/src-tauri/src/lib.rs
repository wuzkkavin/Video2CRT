//! Video2CRT - main library entrypoint.
//!
//! Tauri 2 wiring:
//! - registers Tauri commands invoked from the React UI
//! - exposes the orchestrator that drives the 6-stage CRT pipeline
//! - emits progress events consumed by the React frontend
//!
//! The heavy lifting (ffmpeg / yt-dlp / faster-whisper) is performed by
//! helper functions in [`orchestrator`]. They emit `pipeline://progress`
//! events on every state change so the UI can render a progress bar without
//! polling.

pub mod orchestrator;
mod settings;
mod translator;

use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

/// Pipeline kickoff payload coming from the React UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartJobRequest {
    pub url: String,
    /// Optional override of the project root. When None we fall back to
    /// `~/Documents/Hermes/Video2CRT`.
    pub project_root: Option<String>,
    /// Crop value (gotcha 6 + 24). Default "960:720:160:0" (4:3 pillarbox).
    pub crop: Option<String>,
    /// ASR language hint ("ja", "en", "zh", None => auto).
    pub asr_language: Option<String>,
    /// Whether to generate subtitles at all (default true).
    #[serde(default = "default_true")]
    pub enable_subtitles: bool,
    /// Use cloud translation via `Minimax` (default false).
    pub cloud_translation: bool,
    /// Model id for cloud translation (only used when `cloud_translation=true`).
    pub translation_model: Option<String>,
    /// Optional output directory for `source.mp4 / raw.mp4 / *.srt /
    /// final.mp4`. When Some, files are written into this dir directly
    /// (no `yt_<id>` subfolder created). When None, the orchestrator
    /// falls back to `<projectRoot>/output/yt_<id>/`.
    pub output_dir: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Result of a successful job run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSummary {
    pub video_id: String,
    pub output_dir: String,
    pub final_mp4: String,
    pub srt: String,
}

/// Tauri command: start the pipeline. Returns immediately with a
/// `video_id`; progress is delivered via `pipeline://progress` events.
#[tauri::command]
async fn start_job(
    app: tauri::AppHandle,
    req: StartJobRequest,
) -> Result<orchestrator::JobHandle, String> {
    // Append a breadcrumb to a known log file so the user can confirm the
    // command was reached even if Rust is silently panicking inside the
    // spawned tokio task. Without this, a stuck UI shows "0% / init 已啟動,
    // 等待後端…" forever with no further activity, leaving the user
    // unable to tell whether the IPC call reached Rust at all.
    let breadcrumb = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| "C:\\Users\\Public".to_string());
    let log_path = std::path::PathBuf::from(breadcrumb)
        .join("Documents")
        .join("Hermes")
        .join("Video2CRT")
        .join("video2crt-startup.log");
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map(|mut f| {
            use std::io::Write;
            let _ = writeln!(
                f,
                "[{}] start_job called: url={}, crop={:?}, projectRoot={:?}",
                chrono_like_timestamp(),
                req.url,
                req.crop,
                req.project_root,
            );
        });
    log::info!("start_job called: {:?}", req);
    orchestrator::start(app, req).await.map_err(|e| e.to_string())
}

/// User-visible timestamp in `YYYY-MM-DD HH:MM:SS` format using the
/// host machine's local timezone (whatever Windows has set under the
/// hood). Replaces the original `days=NNNN HH:MM:SSZ` format that was
/// UTC and confused the user who saw "05:21:58Z" instead of their
/// expected 台北 13:21.
///
/// Uses `chrono::Local::now()` directly. `chrono` already handles
/// Windows timezone via the registry + dynamic timezone APIs, so
/// `Local::now()` returns the same wall-clock time the user sees in
/// their task tray. The previous attempt called Win32
/// `GetDynamicTimeZoneInformation` from Rust via an unsafe FFI shim,
/// but the `Bias` field was consistently 0 — meaning the layout of
/// the `DYNAMIC_TIME_ZONE_INFORMATION` struct was wrong (the
/// `StandardDate` field is actually `SYSTEMTIME`, a `u16[8]`, not
/// `u16[16]` as I had it). Rather than chase that, just use
/// `chrono::Local` which gets it right.
fn chrono_like_timestamp() -> String {
    chrono::Local::now()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

/// Tauri command: cancel a running job by video_id.
#[tauri::command]
async fn cancel_job(app: tauri::AppHandle, video_id: String) -> Result<(), String> {
    orchestrator::cancel(&app, &video_id)
        .await
        .map_err(|e| e.to_string())
}

/// Tauri command: open a file/folder in the system file manager (Explorer).
/// Wrapped around `tauri-plugin-opener` so the React side does not have to
/// shell out itself.
#[tauri::command]
async fn reveal_in_explorer(app: tauri::AppHandle, path: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    // `reveal_item_in_dir` opens the parent dir and selects the target on
    // Windows Explorer, matching the "MEDIA:" token behaviour in Hermes.
    app.opener()
        .reveal_item_in_dir(&path)
        .map_err(|e| e.to_string())
}

/// Tauri command: get API key status (true / false). Never returns the key.
#[tauri::command]
fn has_api_key() -> bool {
    settings::has_api_key()
}

/// Tauri command: save API key into Windows Credential Manager.
#[tauri::command]
fn save_api_key(key: String) -> Result<(), String> {
    settings::save_api_key(&key).map_err(|e| e.to_string())
}

/// Tauri command: clear API key from Windows Credential Manager.
#[tauri::command]
fn delete_api_key() -> Result<(), String> {
    settings::delete_api_key().map_err(|e| e.to_string())
}

/// Tauri command: list available cloud translation models. Tries
/// `GET /v1/models` with the saved key first, falls back to a hard-coded
/// list when no key is configured or the request fails.
#[tauri::command]
async fn list_translation_models() -> Vec<translator::ModelInfo> {
    translator::list_models().await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Register a global so commands that need an AppHandle without
            // being passed one (rare) can still reach it.
            app.manage(orchestrator::JobRegistry::default());
            // Fire one startup event so the React side can do "is api key set"
            // without an explicit call.
            let _ = app.emit("pipeline://ready", ());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_job,
            cancel_job,
            reveal_in_explorer,
            has_api_key,
            save_api_key,
            delete_api_key,
            list_translation_models,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Video2CRT app");
}