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
    /// Use cloud translation via `Minimax` (default false).
    pub cloud_translation: bool,
    /// Model id for cloud translation (only used when `cloud_translation=true`).
    pub translation_model: Option<String>,
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

/// User-visible timestamp in YYYY-MM-DD HH:MM:SS format using the
/// host machine's local timezone (whatever Windows has set under the
/// hood). The previous `days=NNNN HH:MM:SSZ` format was UTC and
/// confused the user who saw "05:21:58Z" instead of their expected
/// 台北 13:21. We read the offset directly from the Win32
/// `GetDynamicTimeZoneInformation` API, which is the same source
/// the Windows clock tray uses, so we don't need to chase registry
/// keys or time-zone names.
fn chrono_like_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    // 1. Current epoch seconds (UTC).
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = dur.as_secs();

    // 2. Ask Windows for the local-time UTC offset, in minutes.
    //    The dynamic API correctly handles both standard and DST.
    #[cfg(windows)]
    let offset_minutes: i64 = unsafe {
        let mut tz = std::mem::zeroed::<sysinfoapi::DYNAMIC_TIME_ZONE_INFORMATION>();
        let rc = sysinfoapi::GetDynamicTimeZoneInformation(&mut tz);
        if rc != 0 {
            // `Bias` is in MINUTES WEST of UTC (so for Taipei UTC+8 it
            // is -480). To convert an epoch second to local we ADD
            // (in minutes) the negative of Bias — i.e. local = UTC +
            // (-Bias). Note: Windows reports -480 for Taipei, which
            // means 8 hours EAST of UTC.
            -(*(&tz.Bias) as i64)
        } else {
            0
        }
    };
    #[cfg(not(windows))]
    let offset_minutes: i64 = 0;

    // 3. Apply the offset to get local seconds-since-epoch, then
    //    split into Y/M/D H:M:S.
    let local_secs = (total_secs as i64).wrapping_add(offset_minutes * 60);
    let days = (local_secs / 86400) as i64;
    let secs_today = (local_secs.rem_euclid(86400)) as u64;
    let h = ((secs_today / 3600) % 24) as u32;
    let m = ((secs_today / 60) % 60) as u32;
    let s = (secs_today % 60) as u32;

    // 4. Convert epoch-days to Y/M/D via the proleptic Gregorian
    //    formula (matches `date(1)` on Linux/macOS).
    let z = days + 719468; // days from 0000-03-01
    let era = if z >= 0 { z / 146097 } else { (z - 146096) / 146097 };
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let mth = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if mth <= 2 { y + 1 } else { y };

    format!(
        "{year:04}-{mth:02}-{d:02} {h:02}:{m:02}:{s:02}",
    )
}

#[cfg(windows)]
mod sysinfoapi {
    use std::ffi::c_void;
    #[repr(C)]
    pub(super) struct DYNAMIC_TIME_ZONE_INFORMATION {
        pub Bias: i32,
        pub StandardName: [u16; 32],
        pub StandardDate: [u16; 16],   // SYSTEMTIME, but we only need Bias
        pub StandardBias: i32,
        pub DaylightName: [u16; 32],
        pub DaylightDate: [u16; 16],
        pub DaylightBias: i32,
        pub TimeZoneKeyName: [u16; 128],
        pub DynamicDaylightTimeDisabled: u8,
        _padding: [u8; 3],
    }
    extern "system" {
        pub(super) fn GetDynamicTimeZoneInformation(
            lpTimeZoneInformation: *mut DYNAMIC_TIME_ZONE_INFORMATION,
        ) -> u32;
    }
    #[allow(dead_code)]
    fn _phantom(_: *const c_void) {}
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