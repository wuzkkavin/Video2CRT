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

/// Build a `tokio::process::Command` for the given program with the
/// Windows `CREATE_NO_WINDOW` flag set so child processes don't pop
/// up a console window alongside our GUI. The previous behaviour
/// (spawning without the flag) caused a black cmd window to flash
/// every time the user kicked off a job. The 0x08000000 flag is the
/// standard value for `CREATE_NO_WINDOW` from `WinBase.h`.
#[cfg(windows)]
fn cmd_no_window<S: AsRef<std::ffi::OsStr>>(program: S) -> tokio::process::Command {
    let mut c = tokio::process::Command::new(program);
    c.creation_flags(0x0800_0000);
    c
}

#[cfg(not(windows))]
fn cmd_no_window<S: AsRef<std::ffi::OsStr>>(program: S) -> tokio::process::Command {
    tokio::process::Command::new(program)
}
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};

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

/// Faster-Whisper is CPU and memory intensive. Running two model loads at
/// once has produced nondeterministic NumPy failures, so subtitle inference is
/// deliberately serialized while downloads and CRT rendering remain parallel.
pub struct AsrGate {
    semaphore: Arc<Semaphore>,
}

impl Default for AsrGate {
    fn default() -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(1)),
        }
    }
}

impl AsrGate {
    async fn acquire(&self) -> Result<OwnedSemaphorePermit> {
        self.semaphore
            .clone()
            .acquire_owned()
            .await
            .context("字幕辨識佇列已關閉")
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
        "translate" => 0.05,
        "burn" => 0.05,
        "mux" => 0.05,
        _ => 0.0,
    }
}

/// Cumulative weights so we can compute overall `progress` from the active
/// stage and its intra-stage fraction.
fn cumulative(stage: &str) -> f32 {
    let order = [
        "download",
        "cropdetect",
        "render",
        "asr",
        "translate",
        "burn",
        "mux",
    ];
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

/// Resolve the directory we write the output files into. Order of
/// preference:
///   1. `req.output_dir` if the user picked a folder in OptionsPage.
///   2. The user's Desktop (the default requested for normal app use).
pub fn resolve_output_dir(req: &StartJobRequest, project_root: &Path) -> PathBuf {
    if let Some(p) = &req.output_dir {
        let p = p.trim();
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| project_root.display().to_string());
    PathBuf::from(home).join("Desktop")
}

fn subtitle_mode(req: &StartJobRequest) -> &str {
    match req.subtitle_mode.as_deref() {
        Some("original") => "original",
        Some("none") => "none",
        Some("bilingual") => "bilingual",
        _ if !req.enable_subtitles => "none",
        _ => "bilingual",
    }
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
    // Output dir honors req.output_dir if the user picked a folder
    // in OptionsPage; otherwise fall back to the historical default.
    // We capture it as `base_output_dir` because the final working
    // directory is `base_output_dir / <safe_title>` — user requested
    // 2026-09-14 that the working folder be named after the video
    // title (or the user-picked parent + title), not the bare id.
    let base_output_dir = resolve_output_dir(&req, &project_root);

    // Write breadcrumb log at every key step so we can tell where
    // the synchronous part of `start()` dies (if it does) before
    // the tokio::spawn runs.
    let breadcrumb_log = |line: &str| {
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(
            std::path::PathBuf::from(std::env::var("USERPROFILE").unwrap_or_default())
                .join("Documents")
                .join("Hermes")
                .join("Video2CRT")
                .join("video2crt-startup.log"),
        ) {
            use std::io::Write as _;
            let _ = writeln!(f, "[trace] {line}");
        }
    };
    breadcrumb_log(&format!(
        "A: about to create_dir_all({})",
        base_output_dir.display()
    ));

    // Fetch the video title before reserving the working directory.
    // A metadata failure stops here, so no ID-named fallback directory
    // can be created and no previous result can be overwritten.
    let title = fetch_video_title(&req.url, &video_id).await?;
    breadcrumb_log(&format!("B0: fetched video title: {title}"));
    let sub_dir_name = safe_dirname(&title, &video_id);
    let output_dir = reserve_output_dir(&base_output_dir, &sub_dir_name)?;
    breadcrumb_log(&format!("B0.5: output_dir = {}", output_dir.display()));

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
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(
            std::path::PathBuf::from(std::env::var("USERPROFILE").unwrap_or_default())
                .join("Documents")
                .join("Hermes")
                .join("Video2CRT")
                .join("video2crt-startup.log"),
        ) {
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
    emit_progress(
        &app,
        &handle.video_id,
        "init",
        0.0,
        "starting pipeline",
        None,
    );

    check_cancel(&cancel_flag).await?;

    // Stage 1: yt-dlp download
    stage_begin(
        &app,
        &handle,
        "download",
        "downloading from YouTube via yt-dlp",
    );
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
            if detected_crop.is_empty() {
                "(none)".to_string()
            } else {
                detected_crop.clone()
            },
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
    if subtitle_mode(&req) == "none" {
        // User unchecked "產生字幕" — skip ASR/SRT/burn entirely. This is
        // the fast path for pure-music videos (your 5-minute wait case).
        emit_progress(
            &app,
            &handle.video_id,
            "asr",
            cumulative("asr"),
            "skipped (subtitles disabled)",
            None,
        );
        stage_done(&app, &handle, "asr");
        emit_progress(
            &app,
            &handle.video_id,
            "burn",
            cumulative("burn"),
            "skipped (subtitles disabled)",
            None,
        );
        stage_done(&app, &handle, "burn");
        // Still need to mux raw.mp4 (CRT) + source.mp4 audio → final.mp4
        stage_begin(
            &app,
            &handle,
            "mux",
            "muxing audio + -aspect 16:9 (no subtitles)",
        );
        let raw = PathBuf::from(&handle.output_dir).join("raw.mp4");
        let source = PathBuf::from(&handle.output_dir).join("source.mp4");
        let final_mp4 = PathBuf::from(&handle.output_dir).join("final.mp4");
        // Reuse Python mux helper would require sidecar; do it directly
        // with ffmpeg (same args as mux_audio_local).
        let mux_status = cmd_no_window(locate_ffmpeg().await?)
            .arg("-y")
            .arg("-i")
            .arg(&raw)
            .arg("-i")
            .arg(&source)
            .arg("-map")
            .arg("0:v")
            .arg("-map")
            .arg("1:a")
            .arg("-c:v")
            .arg("copy")
            .arg("-c:a")
            .arg("aac")
            .arg("-b:a")
            .arg("192k")
            .arg("-aspect")
            .arg("16:9")
            .arg("-shortest")
            .arg(&final_mp4)
            .current_dir(Path::new(&handle.output_dir))
            .status()
            .await;
        match mux_status {
            Ok(s) if s.success() => {}
            _ => {
                // Fallback: copy raw.mp4 as final (no audio)
                let _ = tokio::fs::copy(&raw, &final_mp4).await;
            }
        }
        stage_done(&app, &handle, "mux");
    } else {
        stage_begin(
            &app,
            &handle,
            "asr",
            "等待本機語音辨識資源",
        );
        // Only ASR is gated. The existing download, crop, CRT and mux paths
        // intentionally remain unchanged and may still run concurrently.
        let asr_gate = app.state::<AsrGate>();
        let permit = asr_gate.acquire().await?;
        emit_progress(
            &app,
            &handle.video_id,
            "asr",
            cumulative("asr"),
            "running faster-whisper via Python sidecar",
            None,
        );
        let translation_mode = req.translation_mode.as_deref().unwrap_or("local");
        let cloud_fallback_enabled = subtitle_mode(&req) == "bilingual"
            && cloud_translation_enabled(req.cloud_translation, crate::settings::has_api_key());
        if req.cloud_translation && !cloud_fallback_enabled {
            emit_progress(
                &app,
                &handle.video_id,
                "translate",
                cumulative("translate"),
                if translation_mode == "cloud" {
                    "未儲存 MiniMax API Key，無法使用純雲端翻譯"
                } else {
                    "未儲存 MiniMax API Key，將只使用本機繁中翻譯"
                },
                None,
            );
        }
        let payload = serde_json::json!({
            "url": req.url,
            "outputDir": handle.output_dir,
            "videoId": handle.video_id,
            "crop": crop_value,
            "asrLanguage": req.asr_language,
            "subtitleMode": subtitle_mode(&req),
            "translationMode": translation_mode,
            "cloudTranslation": cloud_fallback_enabled,
            "translationModel": req.translation_model,
        });
        run_subtitle_pipeline(&payload, &cancel_flag, |stage, intra, message| {
            emit_progress(
                &app,
                &handle.video_id,
                stage,
                cumulative(stage) + intra * stage_weight(stage),
                message,
                None,
            );
        })
        .await?;
        drop(permit);
    }

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

fn cloud_translation_enabled(requested: bool, has_key: bool) -> bool {
    requested && has_key
}

fn requested_translation_mode(payload: &serde_json::Value) -> &str {
    match payload["translationMode"].as_str() {
        Some("cloud") => "cloud",
        Some("cloudFallback") => "cloudFallback",
        _ => "local",
    }
}

/// Shared by the GUI and real CLI verification. Translation is a mandatory
/// barrier BEFORE finalize/burn. Errors and cancellation cannot emit Done.
pub async fn run_subtitle_pipeline(
    payload: &serde_json::Value,
    cancel_flag: &Arc<Mutex<bool>>,
    progress: impl Fn(&str, f32, &str),
) -> Result<()> {
    let script = locate_sidecar()?;
    let mut step = payload.clone();
    step["phase"] = serde_json::json!("prepare");
    run_sidecar(&script, &step.to_string(), cancel_flag, &progress).await?;
    step["phase"] = serde_json::json!("finalize");
    if requested_translation_mode(payload) == "cloud" {
        if !payload["cloudTranslation"].as_bool().unwrap_or(false) {
            anyhow::bail!("純雲端翻譯需要先在設定中儲存 MiniMax API Key")
        }
        progress("translate", 0.0, "使用已選擇的雲端翻譯");
        let output_dir = payload["outputDir"].as_str().context("missing outputDir")?;
        let model = payload["translationModel"]
            .as_str()
            .filter(|m| !m.trim().is_empty())
            .unwrap_or("MiniMax-M3");
        fill_translations(output_dir, model, cancel_flag, &progress).await?;
        step["translationMode"] = serde_json::json!("cloud");
        return run_sidecar(&script, &step.to_string(), cancel_flag, &progress).await;
    }
    // Local translation is always first. Cloud is an explicit fallback only
    // when the user selected it and a saved key is available.
    step["translationMode"] = serde_json::json!("local");
    match run_sidecar(&script, &step.to_string(), cancel_flag, &progress).await {
        Ok(()) => Ok(()),
        Err(local_error) if requested_translation_mode(payload) == "cloudFallback"
            && payload["cloudTranslation"].as_bool().unwrap_or(false) => {
            progress("translate", 0.0, "本機翻譯失敗，改用已選擇的雲端翻譯");
            let output_dir = payload["outputDir"].as_str().context("missing outputDir")?;
            let model = payload["translationModel"]
                .as_str()
                .filter(|m| !m.trim().is_empty())
                .unwrap_or("MiniMax-M3");
            fill_translations(output_dir, model, cancel_flag, &progress)
                .await
                .with_context(|| format!("本機翻譯失敗（{local_error}），雲端備援也失敗"))?;
            step["translationMode"] = serde_json::json!("cloud");
            run_sidecar(&script, &step.to_string(), cancel_flag, &progress).await
        }
        Err(error) => Err(error),
    }
}

async fn run_sidecar(
    script: &Path,
    payload: &str,
    cancel_flag: &Arc<Mutex<bool>>,
    progress: &impl Fn(&str, f32, &str),
) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    let local = PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default());
    let candidates = [
        local.join("Video2CRT/bin/python.exe"),
        local.join("hermes/hermes-agent/venv/Scripts/python.exe"),
    ];
    let python = candidates
        .into_iter()
        .find(|p| p.is_file())
        .unwrap_or_else(|| PathBuf::from("python"));
    let mut child = cmd_no_window(python)
        .arg(script)
        .arg(payload)
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUTF8", "1")
        .env("HF_HUB_DISABLE_TELEMETRY", "1")
        .env_remove("PYTHONPATH")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .context("無法啟動字幕處理程式")?;
    // Drain stderr concurrently: a full ffmpeg/model pipe must not deadlock.
    let stderr = child.stderr.take().context("missing stderr")?;
    let stderr_task = tokio::spawn(async move {
        let mut tail = String::new();
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            tail.push_str(&line);
            tail.push('\n');
            if tail.len() > 12000 {
                tail = tail
                    .chars()
                    .rev()
                    .take(6000)
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect();
            }
        }
        tail
    });
    let mut lines = BufReader::new(child.stdout.take().context("missing stdout")?).lines();
    let mut poll = tokio::time::interval(std::time::Duration::from_millis(150));
    let mut sidecar_error = None;
    loop {
        tokio::select! {
            _ = poll.tick() => {
                if *cancel_flag.lock().await {
                    // Python may be waiting for ffmpeg. Terminate the process tree.
                    #[cfg(windows)]
                    if let Some(pid) = child.id() {
                        let _ = cmd_no_window("taskkill").args(["/PID", &pid.to_string(), "/T", "/F"]).output().await;
                    }
                    let _ = child.kill().await;
                    stderr_task.abort();
                    anyhow::bail!("cancelled by user");
                }
            }
            line = lines.next_line() => {
                let Some(line) = line? else { break };
                if let Some(json) = line.strip_prefix("PROGRESS ") {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(json) {
                        progress(v["stage"].as_str().unwrap_or("asr"),
                            v["progress"].as_f64().unwrap_or(0.0) as f32,
                            v["message"].as_str().unwrap_or(""));
                    }
                } else if let Some(json) = line.strip_prefix("ERROR ") {
                    sidecar_error = Some(json.to_owned());
                }
            }
        }
    }
    let status = child.wait().await?;
    let stderr = stderr_task.await.unwrap_or_default();
    if let Some(error) = sidecar_error {
        anyhow::bail!("字幕處理失敗：{error}");
    }
    if !status.success() {
        anyhow::bail!("字幕處理失敗 ({status})：{stderr}");
    }
    if *cancel_flag.lock().await {
        anyhow::bail!("cancelled by user");
    }
    Ok(())
}

fn locate_sidecar() -> Result<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    // 1. Compile-time absolute path from Cargo (works in dev builds).
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bin/pipeline_cli.py"));

    // 2. Release-build recovery: walk up from current_exe() looking
    //    for src-tauri/bin/pipeline_cli.py. The .exe lives at
    //    <repo>/video2crt-app/src-tauri/target/release/video2crt.exe,
    //    so we go up 4 levels (release → target → src-tauri →
    //    video2crt-app) then descend into src-tauri/bin/.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("pipeline_cli.py"));
            candidates.push(dir.join("bin").join("pipeline_cli.py"));
            // Walk up to 4 parents looking for src-tauri/bin.
            let mut p = dir.to_path_buf();
            for _ in 0..5 {
                let candidate = p.join("src-tauri").join("bin").join("pipeline_cli.py");
                if candidate.exists() {
                    candidates.push(candidate.clone());
                    break;
                }
                match p.parent() {
                    Some(parent) => p = parent.to_path_buf(),
                    None => break,
                }
            }
        }
    }

    // 3. Cwd-relative fallbacks (kept for completeness).
    candidates.push(PathBuf::from("src-tauri/bin/pipeline_cli.py"));
    candidates.push(PathBuf::from("bin/pipeline_cli.py"));

    for p in &candidates {
        if p.exists() {
            return Ok(p.canonicalize().unwrap_or_else(|_| p.clone()));
        }
    }

    anyhow::bail!(
        "pipeline_cli.py not found. Tried:\n  - {}\n\nThis is a build / packaging issue. The Python sidecar script must\nlive next to the Tauri binary. See HANDOFF-newtask-app.md Phase 2.",
        candidates
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("\n  - ")
    )
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
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(
        std::path::PathBuf::from(std::env::var("USERPROFILE").unwrap_or_default())
            .join("Documents")
            .join("Hermes")
            .join("Video2CRT")
            .join("video2crt-startup.log"),
    ) {
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

/// Preserve the displayed title, replacing only Windows-invalid characters.
fn safe_dirname(raw_title: &str, _video_id: &str) -> String {
    let title: String = raw_title
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '＿',
            c if c.is_control() => ' ',
            c => c,
        })
        .take(120)
        .collect();
    let title = title.trim().trim_end_matches('.');
    let mut title = if title.is_empty() {
        "未命名影片".to_owned()
    } else {
        title.to_owned()
    };
    let stem = title.split('.').next().unwrap_or("").to_ascii_uppercase();
    if ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit())
    {
        title.insert(0, '_');
    }
    title
}

/// Atomically reserve a new title folder; never overwrite a previous conversion.
fn reserve_output_dir(base: &Path, title: &str) -> Result<PathBuf> {
    std::fs::create_dir_all(base)?;
    for number in 1..10000 {
        let name = if number == 1 {
            title.to_owned()
        } else {
            format!("{title} ({number})")
        };
        let candidate = base.join(name);
        match std::fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e).context("無法建立影片標題資料夾"),
        }
    }
    anyhow::bail!("同名影片資料夾過多，請選擇另一個輸出位置")
}

/// Fetch the video title via a metadata-only yt-dlp invocation. We
/// do this BEFORE the real download so we can build the output
/// directory using the human-readable title. Fails open to the
/// video id if the metadata call errors (network blip, age-gate,
/// unavailable) so the pipeline still completes.
async fn fetch_video_title(url: &str, _video_id: &str) -> Result<String> {
    // Try each yt-dlp candidate from the same lookup table
    // download_with_ytdlp uses (kept short here — metadata fetch is
    // best-effort and we don't want a long lookup failure delay).
    let candidates = [
        "C:/Users/asaialabs/AppData/Local/hermes/hermes-agent/venv/Scripts/yt-dlp.exe",
        "C:/Users/asaialabs/AppData/Local/hermes/hermes-agent/Scripts/yt-dlp.exe",
        "yt-dlp",
    ];
    let localappdata = std::env::var("LOCALAPPDATA")
        .unwrap_or_else(|_| "C:/Users/asaialabs/AppData/Local".to_string());
    let node_exe = format!(
        "{}",
        std::path::Path::new(&localappdata)
            .join("hermes")
            .join("node")
            .join("node.exe")
            .to_string_lossy()
    );
    // Bound the whole title-fetch attempt. Playlist URLs
    // (e.g. `&list=RDxxx&start_radio=1`) make yt-dlp try to resolve
    // the entire mix, which can hang indefinitely. If the title cannot
    // be obtained, return an error before creating an output directory.
    let result = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        for bin in &candidates {
            if *bin != "yt-dlp" && !std::path::Path::new(bin).exists() {
                continue;
            }
            let mut cmd = cmd_no_window(bin);
            cmd.env_remove("PYTHONPATH");
            cmd.env("PYTHONIOENCODING", "utf-8").kill_on_drop(true);
            // NO `PYTHONIOENCODING=utf-8` here — yt-dlp's already-bound
            // `sys.stdout` writer ignores it anyway, but setting it
            // makes yt-dlp try to re-encode progress strings as UTF-8
            // and crash with `[Errno 22] Invalid argument` on cp950
            // consoles when a non-cp950 byte (e.g. `×`) shows up.
            // We rely on piped stdout + strict `from_utf8` decode in
            // Rust instead.
            if let Ok(out) = cmd
                .arg("--no-playlist") // never expand `list=RD...` to a mix
                .arg("--skip-download")
                .arg("--encoding")
                .arg("utf-8")
                .arg("--js-runtimes")
                .arg(format!("node:{}", node_exe))
                .arg("--remote-components")
                .arg("ejs:github")
                .arg("--print")
                .arg("%(title)j")
                .arg(url)
                .output()
                .await
            {
                if out.status.success() {
                    // Strict UTF-8 decode. `from_utf8_lossy` would
                    // inject U+FFFD for invalid bytes, and that
                    // replacement character then leaks into the
                    // output directory name as `?` on NTFS.
                    // Decoding failures are non-fatal — we just
                    // fall through to the next candidate.
                    if let Ok(s) = std::str::from_utf8(&out.stdout) {
                        if let Ok(title) = serde_json::from_str::<String>(s.trim()) {
                            if !title.trim().is_empty() {
                                return Some(title);
                            }
                        }
                    }
                }
            }
        }
        None
    })
    .await;
    match result {
        Ok(Some(title)) => Ok(title),
        _ => anyhow::bail!("無法取得 YouTube 影片標題，請確認網址或網路後重試；尚未建立輸出資料夾"),
    }
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

    let mut child = cmd_no_window(bin)
        .arg("-o")
        .arg("source.%(ext)s")
        .current_dir(outdir)
        // Don't pin to mp4 only - YouTube serves vp9/av1/opus/webm as well.
        // Let yt-dlp pick best video+audio and we re-mux to mp4 ourselves.
        .arg("-f")
        .arg("bv*+ba/b")
        .arg("--merge-output-format")
        .arg("mp4")
        .arg("--no-part") // write final file directly so partial files don't satisfy .exists() checks downstream
        .arg("--no-playlist") // user pastes the URL with extra query params
        // like ?start_radio=1 or ?list=...; without --no-playlist
        // yt-dlp enters playlist mode and tries to download the whole
        // list, which (a) is way slower than the user expected and
        // (b) produces an Errno 22 Invalid argument when the playlist
        // entries don't fit the %(ext)s template we set above.
        .arg("--js-runtimes")
        // Modern YouTube serves a JS challenge page that yt-dlp needs
        // to evaluate before extracting formats. Without a JS runtime
        // it falls back to deprecated extraction that errors out on
        // many videos with "This video is unavailable". Tell yt-dlp
        // where to find Node — Hermes bundles Node under
        // %LOCALAPPDATA%\hermes\node\node.exe, and we also fall back
        // to PATH so a system Node install (winget) works too.
        .arg(format!(
            "node:{}",
            std::path::Path::new(
                &std::env::var("LOCALAPPDATA")
                    .unwrap_or_else(|_| "C:/Users/asaialabs/AppData/Local".to_string())
            )
            .join("hermes")
            .join("node")
            .join("node.exe")
            .to_string_lossy()
        ))
        // The remote component lets yt-dlp fetch the EJS challenge
        // solver script from the yt-dlp GitHub release, which lets it
        // solve YouTube's signature challenge. Without this, downloads
        // still succeed but at lower quality (no 1080p / no high-bitrate
        // streams). Tested on 2026-09-14 against three different
        // YouTube IDs (jNQXAC9IVRw, dQw4w9WgXcQ, CaCSuzR4DwM) — without
        // this flag we got 533KB-33MB (low quality); with it we should
        // get the full bitrate.
        .arg("--remote-components")
        .arg("ejs:github")
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
            emit_progress(&app_b, &vid_b, "download", 0.0, "yt-dlp log", Some(&line));
        }
        last_err
    });

    // 10-minute hard timeout so we don't hang forever on stuck downloads
    // (e.g. YouTube 403 / JS-runtime issues).
    let status = match tokio::time::timeout(std::time::Duration::from_secs(600), child.wait()).await
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
            if last_err_line.is_empty() {
                "(no stderr)".to_string()
            } else {
                last_err_line
            }
        );
    }
    Ok(())
}

/// Stage 2: detect pillarbox/letterbox bars. Hint only (gotcha 34).
async fn cropdetect(source: &Path) -> Result<String> {
    let ffmpeg = locate_ffmpeg().await?;
    let out = cmd_no_window(ffmpeg)
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
    // Probe source codec first, then pick the right hw decoder.
    // Previous code forced `-hwaccel cuda -c:v vp9_cuvid` for all inputs,
    // which ACCESS_VIOLATIONs (0xC0000005) on h264/av1/hevc sources.
    let probe = cmd_no_window(&ffmpeg)
        .arg("-v")
        .arg("error")
        .arg("-select_streams")
        .arg("v:0")
        .arg("-show_entries")
        .arg("stream=codec_name")
        .arg("-of")
        .arg("default=nw=1:nk=1")
        .arg(source)
        .output()
        .await;
    let codec = probe
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(
                    String::from_utf8_lossy(&o.stdout)
                        .trim()
                        .to_ascii_lowercase(),
                )
            } else {
                None
            }
        })
        .unwrap_or_default();
    let mut cmd = cmd_no_window(ffmpeg);
    cmd.arg("-y");
    // Only use cuda hwaccel with a matching cuvid decoder; otherwise
    // fall back to software decode. This mirrors `ffprobe → decoder` logic.
    match codec.as_str() {
        "h264" | "avc" => {
            cmd.arg("-hwaccel")
                .arg("cuda")
                .arg("-c:v")
                .arg("h264_cuvid");
        }
        "hevc" | "h265" => {
            cmd.arg("-hwaccel")
                .arg("cuda")
                .arg("-c:v")
                .arg("hevc_cuvid");
        }
        "vp9" => {
            cmd.arg("-hwaccel").arg("cuda").arg("-c:v").arg("vp9_cuvid");
        }
        "av1" => {
            cmd.arg("-hwaccel").arg("cuda").arg("-c:v").arg("av1_cuvid");
        }
        _ => {
            // Unknown codec or probe failed → software decode (always safe)
            cmd.arg("-hwaccel").arg("auto");
        }
    }
    let status = cmd
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

/// Translate structured ASR cues, preserving source text and timestamps.
/// Chinese never calls the API. Failed translations stop before the video burn.
async fn fill_translations(
    output_dir: &str,
    model: &str,
    cancel_flag: &Arc<Mutex<bool>>,
    progress: &impl Fn(&str, f32, &str),
) -> Result<usize> {
    let dir = PathBuf::from(output_dir);
    let segments: Vec<serde_json::Value> = serde_json::from_str(
        &tokio::fs::read_to_string(dir.join("subtitle_segments.json")).await?,
    )?;
    let translation_path = dir.join("subtitle_translations.json");
    let mut translated: serde_json::Map<String, serde_json::Value> =
        match tokio::fs::read_to_string(&translation_path).await {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => serde_json::Map::new(),
        };
    let pending: Vec<_> = segments
        .iter()
        .filter(|s| {
            s["language"].as_str() != Some("zh")
                && s["text"]
                    .as_str()
                    .is_some_and(|text| !translated.contains_key(text))
        })
        .collect();
    for (i, segment) in pending.iter().enumerate() {
        if *cancel_flag.lock().await {
            anyhow::bail!("cancelled by user");
        }
        let text = segment["text"].as_str().context("missing subtitle text")?;
        if !translated.contains_key(text) {
            let future = crate::translator::translate_text(text, model);
            tokio::pin!(future);
            let zh = loop {
                tokio::select! {
                    result = &mut future => { break result?; }
                    _ = tokio::time::sleep(std::time::Duration::from_millis(150)) => {
                        if *cancel_flag.lock().await { anyhow::bail!("cancelled by user"); }
                    }
                }
            };
            translated.insert(text.to_owned(), serde_json::json!(zh));
        }
        progress(
            "translate",
            (i + 1) as f32 / pending.len().max(1) as f32,
            &format!("雲端翻譯 {}/{}", i + 1, pending.len()),
        );
    }
    tokio::fs::write(translation_path, serde_json::to_vec_pretty(&translated)?).await?;
    Ok(translated.len())
}

/// Locate ffmpeg.exe on Windows.
async fn locate_ffmpeg() -> Result<String> {
    for candidate in [
        "ffmpeg",
        "C:/ProgramData/chocolatey/bin/ffmpeg.exe",
        "C:/Program Files/ffmpeg/bin/ffmpeg.exe",
        "C:/Users/asaialabs/AppData/Local/Microsoft/WinGet/Links/ffmpeg.exe",
    ] {
        let probe = cmd_no_window(candidate).arg("-version").output().await;
        if let Ok(out) = probe {
            if out.status.success() {
                return Ok(candidate.to_string());
            }
        }
    }
    anyhow::bail!("ffmpeg not found")
}

#[cfg(test)]
mod subtitle_output_tests {
    use super::*;

    #[test]
    fn missing_api_key_falls_back_to_local_translation() {
        assert!(!cloud_translation_enabled(true, false));
        assert!(!cloud_translation_enabled(false, true));
        assert!(cloud_translation_enabled(true, true));
    }

    #[test]
    fn subtitle_modes_preserve_legacy_bilingual_default() {
        let mut req: StartJobRequest = serde_json::from_value(serde_json::json!({
            "url": "https://example.com", "cloudTranslation": false
        })).unwrap();
        assert_eq!(subtitle_mode(&req), "bilingual");
        req.subtitle_mode = Some("original".into());
        assert_eq!(subtitle_mode(&req), "original");
        req.subtitle_mode = Some("none".into());
        assert_eq!(subtitle_mode(&req), "none");
    }

    #[test]
    fn title_keeps_chinese_spaces_and_valid_punctuation() {
        assert_eq!(
            safe_dirname("初戀 - 回春丹 (Live) & Friends' 100%", "abc"),
            "初戀 - 回春丹 (Live) & Friends' 100%"
        );
        assert_eq!(safe_dirname("A: B / C?", "abc"), "A＿ B ＿ C＿");
        assert_eq!(safe_dirname("CON.txt", "abc"), "_CON.txt");
        assert!(!safe_dirname("", "abc").starts_with("yt_"));
    }

    #[test]
    fn default_output_has_no_id_folder() {
        let req: StartJobRequest = serde_json::from_value(serde_json::json!({
            "url": "https://www.youtube.com/watch?v=abc", "cloudTranslation": false
        }))
        .unwrap();
        assert_eq!(
            resolve_output_dir(&req, Path::new("project")),
            PathBuf::from(std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap()).join("Desktop")
        );
    }

    #[test]
    fn existing_title_is_never_overwritten() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../test-out/title-contract")
            .join(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
                    .to_string(),
            );
        let first = reserve_output_dir(&root, "中文影片").unwrap();
        std::fs::write(first.join("keep.txt"), "original").unwrap();
        let second = reserve_output_dir(&root, "中文影片").unwrap();
        assert_eq!(second.file_name().unwrap(), "中文影片 (2)");
        assert_eq!(
            std::fs::read_to_string(first.join("keep.txt")).unwrap(),
            "original"
        );
    }

    #[tokio::test]
    async fn chinese_cloud_mode_does_not_need_a_key() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../test-out/cloud-chinese")
            .join(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
                    .to_string(),
            );
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("subtitle_segments.json"),
            r#"[{"language":"zh","text":"你好"}]"#,
        )
        .unwrap();
        let count = fill_translations(
            root.to_str().unwrap(),
            "unused",
            &Arc::new(Mutex::new(false)),
            &|_, _, _| {},
        )
        .await
        .unwrap();
        assert_eq!(count, 0);
        assert_eq!(
            std::fs::read_to_string(root.join("subtitle_translations.json")).unwrap(),
            "{}"
        );
    }
}
