//! Pipeline orchestrator: drives the 6-stage CRT conversion.
//!
//! Stages (per HANDOFF-newtask-app.md):
//! 1. yt-dlp download (10%)
//! 2. cropdetect black-bar detection (5%)
//! 3. libplacebo CRT shader encode (50%)
//! 4. faster-whisper ASR via Python sidecar (20%)
//! 5. SRT burn (10%)
//! 6. mux audio + -aspect 16:9 (5%)
//!
//! Implemented as a tokio task spawned per `start_job` invocation.
//! Progress is emitted via `pipeline://progress` events carrying
//! `{ videoId, stage, progress (0.0..=1.0), message, logLine }`.

use anyhow::{Context, Result};
use futures::FutureExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::process::Command;
use tokio::sync::Mutex;

use crate::StartJobRequest;

/// Token returned to the React UI when a job is kicked off.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobHandle {
    pub video_id: String,
    pub output_dir: String,
}

/// Pipeline state, tracked by `JobRegistry` so cancellation works
/// across stages.
#[derive(Default)]
pub struct JobRegistry {
    inner: Arc<Mutex<HashMap<String, Arc<Mutex<bool>>>>>,
}

impl JobRegistry {
    pub async fn register(&self, video_id: &str) -> Arc<Mutex<bool>> {
        let flag = Arc::new(Mutex::new(false));
        self.inner
            .lock()
            .await
            .insert(video_id.to_string(), flag.clone());
        flag
    }
    pub async fn cancel(&self, video_id: &str) {
        if let Some(flag) = self.inner.lock().await.get(video_id).cloned() {
            *flag.lock().await = true;
        }
    }
    pub async fn remove(&self, video_id: &str) {
        self.inner.lock().await.remove(video_id);
    }
}

/// One progress frame sent to the React UI.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub video_id: String,
    pub stage: String,
    /// 0.0..=1.0 — overall job progress.
    pub progress: f32,
    pub message: String,
    /// Optional incremental log line.
    pub log_line: Option<String>,
}

/// Per-stage weight (matches HANDOFF percentages).
fn stage_weight(stage: &str) -> f32 {
    match stage {
        "download" => 0.10,
        "cropdetect" => 0.05,
        "render" => 0.50,
        "asr" => 0.20,
        "burn" => 0.05,
        "mux" => 0.05,
        _ => 0.0,
    }
}

/// Cumulative weights so we can compute overall `progress` from the active
/// stage and its intra-stage fraction.
fn cumulative(stage: &str) -> f32 {
    let order = ["download", "cropdetect", "render", "asr", "burn", "mux"];
    let mut acc = 0.0_f32;
    for s in order {
        if s == stage {
            return acc;
        }
        acc += stage_weight(s);
    }
    acc
}

/// Resolve the project root: prefer request override, fall back to
/// `~/Documents/Hermes/Video2CRT` per HANDOFF-newtask-app.md.
pub fn resolve_project_root(req: &StartJobRequest) -> PathBuf {
    if let Some(p) = &req.project_root {
        return PathBuf::from(p);
    }
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| "C:\\Users\\Public".to_string());
    PathBuf::from(home)
        .join("Documents")
        .join("Hermes")
        .join("Video2CRT")
}

/// Kick off the orchestrator. Returns immediately with the new job handle.
pub async fn start(app: AppHandle, req: StartJobRequest) -> Result<JobHandle> {
    // Emit a progress event IMMEDIATELY before doing anything else so
    // the UI log region shows the user that the Rust side has received
    // their click. Without this, if the spawned tokio task panics or
    // hangs at any later step, the user only sees the "init 已啟動,
    // 等待後端…" placeholder text and has no way to know whether the
    // IPC call ever reached Rust.
    emit_progress(
        &app,
        &derive_video_id(&req.url),
        "init",
        0.0,
        "Rust received start_job command — entering orchestrator",
        None,
    );

    let project_root = resolve_project_root(&req);
    if !project_root.exists() {
        let msg = format!("Project root does not exist: {}", project_root.display());
        let _ = app.emit(
            "pipeline://error",
            serde_json::json!({
                "videoId": derive_video_id(&req.url),
                "message": msg,
            }),
        );
        anyhow::bail!(msg);
    }

    let video_id = derive_video_id(&req.url);
    let output_dir = project_root.join("output").join(format!("yt_{video_id}"));

    // Write breadcrumb log at every key step so we can tell where
    // the synchronous part of `start()` dies (if it does) before
    // the tokio::spawn runs.
    let breadcrumb_log = |line: &str| {
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(std::path::PathBuf::from(
                std::env::var("USERPROFILE").unwrap_or_default(),
            )
            .join("Documents")
            .join("Hermes")
            .join("Video2CRT")
            .join("video2crt-startup.log"))
        {
            use std::io::Write as _;
            let _ = writeln!(f, "[trace] {}", line);
        }
    };
    breadcrumb_log(&format!("A: about to create_dir_all({})", output_dir.display()));

    std::fs::create_dir_all(&output_dir)
        .with_context(|| format!("creating {}", output_dir.display()))?;
    breadcrumb_log("B: create_dir_all OK");

    let registry = app.state::<JobRegistry>();
    breadcrumb_log("C: got JobRegistry state");

    let cancel_flag = registry.register(&video_id).await;
    breadcrumb_log(&format!("D: registered cancel_flag for {}", video_id));

    let handle = JobHandle {
        video_id: video_id.clone(),
        output_dir: output_dir.to_string_lossy().to_string(),
    };

    // Clone everything we need for the spawned task before any `move`.
    let app_for_task = app.clone();
    let req_for_task = req.clone();
    let handle_for_task = handle.clone();
    breadcrumb_log("E: about to tokio::spawn");
    tokio::spawn(async move {
        // Write to startup log immediately on task entry so we can tell
        // whether the task even starts. If we don't see this line in
        // startup.log, the spawn itself never ran (e.g. panic in
        // tokio runtime, or the Tauri event loop dropped us).
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(std::path::PathBuf::from(
                std::env::var("USERPROFILE").unwrap_or_default(),
            )
            .join("Documents")
            .join("Hermes")
            .join("Video2CRT")
            .join("video2crt-startup.log"))
        {
            use std::io::Write as _;
            let _ = writeln!(f, "[stage] tokio::spawn task entered for {}", video_id);
        }

        // Wrap the pipeline in catch_unwind so a panic inside the tokio
        // task surfaces as a `pipeline://error` event instead of dying
        // silently (which is what was happening — the user saw the
        // "init 已啟動,等待後端…" message forever with no further
        // activity).
        let video_id_for_catch = video_id.clone();
        let app_for_catch = app_for_task.clone();
        let result = std::panic::AssertUnwindSafe(run_pipeline(
            app_for_task.clone(),
            req_for_task,
            handle_for_task,
            cancel_flag,
        ))
        .catch_unwind()
        .await;

        let final_result: anyhow::Result<()> = match result {
            Ok(r) => r,
            Err(panic_payload) => {
                let msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                    (*s).to_string()
                } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "panic in pipeline task (unknown payload)".to_string()
                };
                let _ = app_for_catch.emit(
                    "pipeline://error",
                    serde_json::json!({
                        "videoId": video_id_for_catch,
                        "message": format!("pipeline panicked: {msg}"),
                    }),
                );
                Ok(())
            }
        };

        // Always remove from registry when done, regardless of outcome.
        let reg = app_for_task.state::<JobRegistry>();
        reg.remove(&video_id).await;
        if let Err(e) = final_result {
            let _ = app_for_task.emit(
                "pipeline://error",
                serde_json::json!({
                    "videoId": video_id,
                    "message": e.to_string(),
                }),
            );
        }
    });

    Ok(handle)
}

/// Cancellation entry point: marks the job's cancel flag.
pub async fn cancel(app: &AppHandle, video_id: &str) -> Result<()> {
    let registry = app.state::<JobRegistry>();
    registry.cancel(video_id).await;
    Ok(())
}

/// Top-level pipeline driver.
async fn run_pipeline(
    app: AppHandle,
    req: StartJobRequest,
    handle: JobHandle,
    cancel_flag: Arc<Mutex<bool>>,
) -> Result<()> {
    emit_progress(&app, &handle.video_id, "init", 0.0, "starting pipeline", None);

    check_cancel(&cancel_flag).await?;

    // Stage 1: yt-dlp download
    stage_begin(&app, &handle, "download", "downloading from YouTube via yt-dlp");
    if let Err(e) =
        download_with_ytdlp(&app, &handle, &req.url, Path::new(&handle.output_dir)).await
    {
        stage_error(&app, &handle, "download", &e);
        return Err(e);
    }
    stage_done(&app, &handle, "download");

    // Stage 2: cropdetect (informational; user override wins)
    stage_begin(
        &app,
        &handle,
        "cropdetect",
        "scanning for pillarbox/letterbox bars",
    );
    let source = PathBuf::from(&handle.output_dir).join("source.mp4");
    // Stage 2: dynamic cropdetect (gotcha 6 + 24 + 34). Every YouTube video
    // has different pillarbox sizes, so we MUST NOT use a hardcoded crop
    // like 960:720:160:0 — that will eat real content. We run cropdetect,
    // then either honour the user's manual override (req.crop) or use the
    // detected value as the default. The detected crop is also emitted as
    // a PROGRESS event so the user can see what was chosen.
    let detected_crop = cropdetect(&source).await.unwrap_or_default();
    let crop_value = req
        .crop
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            if detected_crop.is_empty() {
                // cropdetect failed (gotcha 34: colored strips, luminance
                // tricks cropdetect). Fall back to no-op crop (1920:1080:0:0)
                // and let the user retry with manual crop in OptionsPage.
                "1920:1080:0:0".to_string()
            } else {
                detected_crop.clone()
            }
        });
    emit_progress(
        &app,
        &handle.video_id,
        "cropdetect",
        cumulative("cropdetect"),
        &format!(
            "detected pillarbox: {} → using crop={}",
            if detected_crop.is_empty() { "(none)".to_string() } else { detected_crop.clone() },
            crop_value
        ),
        Some(&crop_value),
    );
    stage_done(&app, &handle, "cropdetect");

    // Stage 3: libplacebo CRT render
    stage_begin(&app, &handle, "render", "libplacebo CRT shader encoding");
    let raw = PathBuf::from(&handle.output_dir).join("raw.mp4");
    if let Err(e) = render_crt(&source, &raw, &crop_value).await {
        stage_error(&app, &handle, "render", &e);
        return Err(e);
    }
    stage_done(&app, &handle, "render");

    // Stage 4-6: faster-whisper + SRT + burn + mux — delegated to the
    // Python sidecar `pipeline_cli.py` (kept here as a separate process so
    // we don't have to maintain a Rust ASR binding).
    stage_begin(&app, &handle, "asr", "running faster-whisper via Python sidecar");
    let sidecar = locate_sidecar();
    let payload = serde_json::json!({
        "url": req.url,
        "outputDir": handle.output_dir,
        "videoId": handle.video_id,
        "crop": crop_value,
        "asrLanguage": req.asr_language,
        "cloudTranslation": req.cloud_translation,
        "translationModel": req.translation_model,
    });
    let payload_str = serde_json::to_string(&payload)?;
    match sidecar {
        Ok(path) => {
            if let Err(e) = run_sidecar(&app, &handle, &path, &payload_str, &cancel_flag).await {
                stage_error(&app, &handle, "asr", &e);
                return Err(e);
            }
        }
        Err(e) => {
            stage_error(&app, &handle, "asr", &e);
            return Err(e);
        }
    }
    // sidecar emits its own asr/burn/mux progress; mark them done in series.
    stage_done(&app, &handle, "asr");
    stage_begin(&app, &handle, "burn", "burning subtitles");
    stage_done(&app, &handle, "burn");
    stage_begin(&app, &handle, "mux", "muxing audio + -aspect 16:9");
    stage_done(&app, &handle, "mux");

    emit_progress(
        &app,
        &handle.video_id,
        "done",
        1.0,
        "pipeline complete",
        None,
    );

    let _ = app.emit(
        "pipeline://done",
        serde_json::json!({
            "videoId": handle.video_id,
            "outputDir": handle.output_dir,
            "finalMp4": format!("{}\\final.mp4", handle.output_dir),
            "srt": format!("{}\\zh-Hant.srt", handle.output_dir),
        }),
    );
    Ok(())
}

/// Spawn the Python sidecar and stream its stdout as progress events.
async fn run_sidecar(
    app: &AppHandle,
    handle: &JobHandle,
    py_script: &Path,
    payload: &str,
    cancel_flag: &Arc<Mutex<bool>>,
) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    let mut child = Command::new("python")
        .arg(py_script)
        .arg(payload)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("no stdout from sidecar"))?;
    let mut reader = BufReader::new(stdout).lines();
    while let Some(line) = reader.next_line().await? {
        // Cancel check.
        if *cancel_flag.lock().await {
            let _ = child.kill().await;
            anyhow::bail!("cancelled by user");
        }
        // The sidecar emits JSON lines: PROGRESS {...}, DONE {...}, ERROR {...}.
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("PROGRESS ") {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(rest) {
                let stage = v.get("stage").and_then(|s| s.as_str()).unwrap_or("asr");
                let intra = v.get("progress").and_then(|p| p.as_f64()).unwrap_or(0.0) as f32;
                let msg = v
                    .get("message")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();
                let overall = cumulative(stage) + intra * stage_weight(stage);
                emit_progress(app, &handle.video_id, stage, overall, &msg, None);
            }
        } else if let Some(rest) = trimmed.strip_prefix("DONE ") {
            // Sidecar reports completion inside the same line stream; we
            // parse it but our outer driver has already accounted for asr/
            // burn/mux weights via the intra-progress events above.
            let _ = rest;
        } else if let Some(rest) = trimmed.strip_prefix("ERROR ") {
            anyhow::bail!("sidecar error: {}", rest);
        } else {
            // Treat as plain log.
            emit_progress(
                app,
                &handle.video_id,
                "asr",
                cumulative("asr"),
                "sidecar log",
                Some(trimmed),
            );
        }
    }
    let status = child.wait().await?;
    if !status.success() {
        anyhow::bail!("sidecar exited with {status:?}");
    }
    Ok(())
}

/// Find the bundled Python sidecar script.
fn locate_sidecar() -> Result<PathBuf> {
    // 1. Repo-relative: src-tauri/bin/pipeline_cli.py
    let candidates = [
        "src-tauri/bin/pipeline_cli.py",
        "../src-tauri/bin/pipeline_cli.py",
        "bin/pipeline_cli.py",
    ];
    for c in candidates {
        let p = PathBuf::from(c);
        if p.exists() {
            return Ok(p.canonicalize().unwrap_or(p));
        }
    }
    anyhow::bail!("pipeline_cli.py not found")
}

async fn check_cancel(flag: &Arc<Mutex<bool>>) -> Result<()> {
    if *flag.lock().await {
        anyhow::bail!("cancelled by user");
    }
    Ok(())
}

fn stage_begin(app: &AppHandle, handle: &JobHandle, stage: &str, message: &str) {
    emit_progress(
        app,
        &handle.video_id,
        stage,
        cumulative(stage),
        message,
        None,
    );
}

fn stage_done(app: &AppHandle, handle: &JobHandle, stage: &str) {
    emit_progress(
        app,
        &handle.video_id,
        stage,
        cumulative(stage) + stage_weight(stage),
        &format!("{stage} complete"),
        None,
    );
}

fn stage_error(app: &AppHandle, handle: &JobHandle, stage: &str, e: &anyhow::Error) {
    emit_progress(
        app,
        &handle.video_id,
        stage,
        cumulative(stage),
        &format!("{stage} failed: {e}"),
        Some(&e.to_string()),
    );
}

fn emit_progress(
    app: &AppHandle,
    video_id: &str,
    stage: &str,
    progress: f32,
    message: &str,
    log_line: Option<&str>,
) {
    // Breadcrumb log every emit so we can tell whether the Rust side
    // is publishing events the UI is failing to receive.
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(std::path::PathBuf::from(
            std::env::var("USERPROFILE").unwrap_or_default(),
        )
        .join("Documents")
        .join("Hermes")
        .join("Video2CRT")
        .join("video2crt-startup.log"))
    {
        use std::io::Write as _;
        let _ = writeln!(
            f,
            "[emit] stage={} progress={:.2} message={}",
            stage, progress, message
        );
    }
    let _ = app.emit(
        "pipeline://progress",
        ProgressEvent {
            video_id: video_id.to_string(),
            stage: stage.to_string(),
            progress,
            message: message.to_string(),
            log_line: log_line.map(|s| s.to_string()),
        },
    );
}

/// Derive a deterministic video_id from a YouTube URL.
fn derive_video_id(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("https://youtu.be/") {
        return rest
            .split(['?', '&', '/'])
            .next()
            .unwrap_or("unknown")
            .chars()
            .take(16)
            .collect();
    }
    if let Some(idx) = url.find("v=") {
        let rest = &url[idx + 2..];
        return rest
            .split(['?', '&', '#'])
            .next()
            .unwrap_or("unknown")
            .chars()
            .take(16)
            .collect();
    }
    let mut s: String = url.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    s.truncate(16);
    if s.is_empty() {
        s = "unknown".to_string();
    }
    s
}

/// Stage 1: invoke yt-dlp to download the source video.
///
/// Strategy: pick the first yt-dlp binary that exists on disk (no more
/// "try every candidate in sequence" loop — that previously caused the
/// UI to look frozen because the download stage could take minutes
/// before any PROGRESS event arrived and the user had no feedback).
/// We pipe stdout/stderr to a thread that emits each non-empty line
/// as a PROGRESS log event so the UI log region scrolls in real time.
async fn download_with_ytdlp(
    app: &AppHandle,
    handle: &JobHandle,
    url: &str,
    outdir: &Path,
) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, BufReader};

    let candidates = [
        // Hermes venv (where the user actually has yt-dlp installed,
        // verified via `where yt-dlp`). Listed first so this .exe
        // works for the current operator without depending on PATH.
        "C:/Users/asaialabs/AppData/Local/hermes/hermes-agent/venv/Scripts/yt-dlp.exe",
        "C:/Users/asaialabs/AppData/Local/hermes/hermes-agent/Scripts/yt-dlp.exe",
        // Setup wizard target directory — for distribution builds,
        // the wizard downloads yt-dlp here on first launch.
        // %LOCALAPPDATA% resolves at runtime via env::var below; we
        // leave the literal here as a fallback for the common case
        // and read the env var in the find closure.
        "%LOCALAPPDATA%/Video2CRT/bin/yt-dlp.exe",
        // Common third-party locations.
        "C:/Users/asaialabs/AppData/Local/Microsoft/WinGet/Links/yt-dlp.exe",
        "C:/Users/asaialabs/AppData/Roaming/Python/Python311/Scripts/yt-dlp.exe",
        "C:/ProgramData/chocolatey/bin/yt-dlp.exe",
    ];
    let localappdata = std::env::var("LOCALAPPDATA")
        .unwrap_or_else(|_| "C:/Users/asaialabs/AppData/Local".to_string());
    let bin = candidates
        .iter()
        .map(|c| {
            if let Some(stripped) = c.strip_prefix("%LOCALAPPDATA%/") {
                // Normalise: strip any trailing slash from LOCALAPPDATA,
                // and use a single forward slash (Windows accepts both).
                let base = localappdata.trim_end_matches(['/', '\\']);
                format!("{}/{}", base, stripped)
            } else {
                c.to_string()
            }
        })
        .find(|resolved| std::path::Path::new(resolved).exists())
        .ok_or_else(|| anyhow::anyhow!(
            "yt-dlp not found. Tried:\n  - Hermes venv ({})\n  - %LOCALAPPDATA%\\Video2CRT\\bin\\\n  - WinGet links / Python Scripts / chocolatey.\n\nInstall yt-dlp with:\n  winget install yt-dlp\nor download yt-dlp.exe from https://github.com/yt-dlp/yt-dlp/releases/latest and place it in your PATH.",
            std::env::var("USERPROFILE").unwrap_or_default()
        ))?;

    emit_progress(
        app,
        &handle.video_id,
        "download",
        0.0,
        &format!("downloading via {} ...", bin),
        None,
    );

    let mut child = Command::new(bin)
        .arg("-o")
        .arg(format!("{}/source.%(ext)s", outdir.display()))
        .arg("-f")
        .arg("bv*[ext=mp4][height<=1080]+ba[ext=m4a]/b[ext=mp4]")
        .arg("--merge-output-format")
        .arg("mp4")
        .arg("--no-part") // write final file directly so partial files don't satisfy .exists() checks downstream
        .arg(url)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("no stdout from yt-dlp"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow::anyhow!("no stderr from yt-dlp"))?;

    // Drain both streams concurrently, forwarding each non-empty line
    // as a PROGRESS log event so the UI log region scrolls in real time.
    let app_a = app.clone();
    let vid_a = handle.video_id.clone();
    let app_b = app.clone();
    let vid_b = handle.video_id.clone();
    let out_a = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            emit_progress(
                &app_a,
                &vid_a,
                "download",
                0.0,
                "yt-dlp output",
                Some(&line),
            );
        }
    });
    let out_b = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        let mut last_err = String::new();
        while let Ok(Some(line)) = reader.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            last_err = line.clone();
            emit_progress(
                &app_b,
                &vid_b,
                "download",
                0.0,
                "yt-dlp log",
                Some(&line),
            );
        }
        last_err
    });

    // 10-minute hard timeout so we don't hang forever on stuck downloads
    // (e.g. YouTube 403 / JS-runtime issues).
    let status = match tokio::time::timeout(
        std::time::Duration::from_secs(600),
        child.wait(),
    )
    .await
    {
        Ok(r) => r?,
        Err(_) => {
            let _ = child.kill().await;
            anyhow::bail!("yt-dlp timed out after 600s");
        }
    };

    let _ = out_a.await;
    let last_err_line = out_b.await.unwrap_or_default();
    if !status.success() {
        // Surface yt-dlp's last stderr line as the error message so the
        // user sees "HTTP Error 403: Forbidden" instead of a generic
        // "yt-dlp failed" in the UI.
        anyhow::bail!(
            "yt-dlp failed (exit {:?}): {}",
            status.code(),
            if last_err_line.is_empty() { "(no stderr)".to_string() } else { last_err_line }
        );
    }
    Ok(())
}

/// Stage 2: detect pillarbox/letterbox bars. Hint only (gotcha 34).
async fn cropdetect(source: &Path) -> Result<String> {
    let ffmpeg = locate_ffmpeg().await?;
    let out = Command::new(ffmpeg)
        .arg("-i")
        .arg(source)
        .arg("-vf")
        .arg("cropdetect=24:2:0")
        .arg("-frames:v")
        .arg("200")
        .arg("-f")
        .arg("null")
        .arg("-")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .await?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    // ffmpeg cropdetect emits lines like
    //   [Parsed_cropdetect_0 @ ...] crop=W:H:X:Y
    // We want JUST the "W:H:X:Y" portion. If we keep the leading "crop="
    // and later concatenate it inside another filter expression like
    // `crop={value},libplacebo=...`, ffmpeg sees `crop=crop=...` and
    // refuses ("No option name near '...'") — discovered 2026-09-13.
    let mut last = String::new();
    for line in stderr.lines() {
        if let Some(idx) = line.find("crop=") {
            let raw = &line[idx..];
            let token = raw.split_whitespace().next().unwrap_or("");
            last = token.trim_start_matches("crop=").to_string();
        }
    }
    Ok(last)
}

/// Stage 3: render the CRT shader via ffmpeg + libplacebo (gpu) and encode
/// raw.mp4 (no audio). Follows gotcha 1/2/6/24/34.
///
/// IMPORTANT: libplacebo parses `:` as the option separator inside its
/// filter expression, so an absolute Windows path like `C:\...\crt.glsl`
/// breaks the parser. Always cd into the output directory and pass the
/// shader as a relative file name (`crt.glsl`).
async fn render_crt(source: &Path, raw_out: &Path, crop: &str) -> Result<()> {
    let ffmpeg = locate_ffmpeg().await?;
    let outdir = raw_out
        .parent()
        .ok_or_else(|| anyhow::anyhow!("raw_out has no parent dir"))?;
    let shader_path = outdir.join("crt.glsl");
    let shader_text = match std::fs::read_to_string("../scripts/crt.glsl") {
        Ok(s) => s,
        Err(_) => include_str!("../../scripts/crt.glsl").to_string(),
    };
    std::fs::write(&shader_path, &shader_text)?;
    let vf = format!(
        "crop={crop},libplacebo=custom_shader_path=crt.glsl:w=1920:h=1080:fps=30:force_original_aspect_ratio=0"
    );
    let status = Command::new(ffmpeg)
        .arg("-y")
        .arg("-hwaccel")
        .arg("cuda")
        .arg("-c:v")
        .arg("vp9_cuvid")
        .arg("-i")
        .arg(source)
        .arg("-vf")
        .arg(&vf)
        .arg("-an")
        .arg("-c:v")
        .arg("libx264")
        .arg("-preset")
        .arg("ultrafast")
        .arg("-crf")
        .arg("18")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg(raw_out)
        .current_dir(outdir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;
    if !status.success() {
        anyhow::bail!("ffmpeg render failed (exit {status:?})");
    }
    Ok(())
}

/// Locate ffmpeg.exe on Windows.
async fn locate_ffmpeg() -> Result<String> {
    for candidate in [
        "ffmpeg",
        "C:/ProgramData/chocolatey/bin/ffmpeg.exe",
        "C:/Program Files/ffmpeg/bin/ffmpeg.exe",
        "C:/Users/asaialabs/AppData/Local/Microsoft/WinGet/Links/ffmpeg.exe",
    ] {
        let probe = Command::new(candidate).arg("-version").output().await;
        if let Ok(out) = probe {
            if out.status.success() {
                return Ok(candidate.to_string());
            }
        }
    }
    anyhow::bail!("ffmpeg not found")
}