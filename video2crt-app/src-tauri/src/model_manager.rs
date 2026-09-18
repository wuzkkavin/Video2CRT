//! Optional high-quality local model installation.
//!
//! The installer ships a small CPU-friendly model so the application is
//! immediately usable offline.  This module owns the explicit, user-approved
//! download of the larger Whisper and translation models.  Downloads are
//! pinned to immutable Hugging Face revisions, written to `.part` files, and
//! only become active after every required file has been downloaded and a
//! manifest has been written.

use std::path::{Path, PathBuf};
use std::sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex};

use anyhow::{Context, Result};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const MODEL_VERSION: u32 = 1;
const ESTIMATED_BYTES: u64 = 3_200_000_000;

const ASR_REPO: &str = "mobiuslabsgmbh/faster-whisper-large-v3-turbo";
const ASR_REVISION: &str = "0a363e9161cbc7ed1431c9597a8ceaf0c4f78fcf";
const ASR_FILES: &[&str] = &["config.json", "model.bin", "tokenizer.json", "vocabulary.txt"];

const TRANSLATION_REPO: &str = "jncraton/m2m100_1.2B-ct2-int8";
const TRANSLATION_REVISION: &str = "e50078df6be13a88592b70ea42f4d74c2082e448";
const TRANSLATION_FILES: &[&str] = &[
    "config.json",
    "model.bin",
    "shared_vocabulary.json",
    "sentencepiece.bpe.model",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LargeModelStatus {
    pub available: bool,
    pub asr_ready: bool,
    pub translation_ready: bool,
    pub estimated_bytes: u64,
    pub installed_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInstallProgress {
    pub stage: String,
    pub file: String,
    pub progress: f32,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    version: u32,
    asr: Vec<FileRecord>,
    translation: Vec<FileRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileRecord {
    path: String,
    bytes: u64,
    sha256: String,
}

#[derive(Default)]
pub struct InstallState {
    active: Mutex<Option<Arc<AtomicBool>>>,
}

pub fn model_root(app: &AppHandle) -> Result<PathBuf> {
    app.path()
        .app_local_data_dir()
        .context("無法取得 Video2CRT 使用者資料夾")
        .map(|root| root.join("models").join("large"))
}

fn fallback_model_root() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|root| root.join("Video2CRT").join("models").join("large"))
}

/// Return an installed large-model directory only when its complete manifest
/// and all expected files are present.  The sidecar uses this before the
/// bundled standard model, so an optional install takes effect automatically.
pub fn installed_model_dir(kind: &str) -> Option<PathBuf> {
    let root = fallback_model_root()?;
    let manifest_path = root.join("manifest.json");
    let manifest: Manifest = serde_json::from_slice(&std::fs::read(manifest_path).ok()?).ok()?;
    if manifest.version != MODEL_VERSION {
        return None;
    }
    let records = match kind {
        "asr" => &manifest.asr,
        "translation" => &manifest.translation,
        _ => return None,
    };
    if records.is_empty() || records.iter().any(|record| {
        let path = root.join(kind).join(&record.path);
        !path.is_file() || std::fs::metadata(path).map(|m| m.len() != record.bytes).unwrap_or(true)
    }) {
        return None;
    }
    Some(root.join(kind))
}

fn records_ready(root: &Path, kind: &str, records: &[FileRecord]) -> bool {
    !records.is_empty() && records.iter().all(|record| {
        let path = root.join(kind).join(&record.path);
        path.is_file() && std::fs::metadata(path).map(|m| m.len() == record.bytes).unwrap_or(false)
    })
}

pub fn status_for_root(root: &Path) -> LargeModelStatus {
    let manifest_path = root.join("manifest.json");
    let manifest = std::fs::read(manifest_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Manifest>(&bytes).ok());
    let (asr_ready, translation_ready, installed_bytes) = match manifest {
        Some(manifest) if manifest.version == MODEL_VERSION => {
            let asr = records_ready(root, "asr", &manifest.asr);
            let translation = records_ready(root, "translation", &manifest.translation);
            let bytes = manifest
                .asr
                .iter()
                .filter(|record| root.join("asr").join(&record.path).is_file())
                .map(|record| record.bytes)
                .chain(manifest.translation.iter()
                    .filter(|record| root.join("translation").join(&record.path).is_file())
                    .map(|record| record.bytes))
                .sum();
            (asr, translation, bytes)
        }
        _ => (false, false, 0),
    };
    LargeModelStatus {
        available: asr_ready && translation_ready,
        asr_ready,
        translation_ready,
        estimated_bytes: ESTIMATED_BYTES,
        installed_bytes,
    }
}

fn emit_progress(app: &AppHandle, progress: ModelInstallProgress) {
    let _ = app.emit("model://progress", progress);
}

fn ensure_not_cancelled(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        anyhow::bail!("模型安裝已取消");
    }
    Ok(())
}

async fn sha256_file(path: &Path) -> Result<String> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer).await?;
        if read == 0 { break; }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

async fn download_file(
    app: &AppHandle,
    client: &reqwest::Client,
    root: &Path,
    stage: &str,
    repo: &str,
    revision: &str,
    file_name: &str,
    cancel: &AtomicBool,
    completed_bytes: &mut u64,
) -> Result<FileRecord> {
    ensure_not_cancelled(cancel)?;
    let destination = root.join(stage).join(file_name);
    let part = PathBuf::from(format!("{}.part", destination.display()));
    if let Some(parent) = destination.parent() { tokio::fs::create_dir_all(parent).await?; }

    let url = format!("https://huggingface.co/{repo}/resolve/{revision}/{file_name}?download=true");
    let response = client.get(url).send().await?.error_for_status()?;
    let total = response.content_length().unwrap_or(0);
    let mut stream = response.bytes_stream();
    let mut file = tokio::fs::File::create(&part).await?;
    let mut downloaded = 0u64;
    while let Some(chunk) = stream.next().await {
        ensure_not_cancelled(cancel)?;
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        let fraction = if total == 0 { 0.0 } else { downloaded as f32 / total as f32 };
        emit_progress(app, ModelInstallProgress {
            stage: stage.to_string(),
            file: file_name.to_string(),
            progress: fraction,
            downloaded_bytes: *completed_bytes + downloaded,
            total_bytes: ESTIMATED_BYTES,
            message: format!("下載 {stage} 模型：{file_name}"),
        });
    }
    file.flush().await?;
    ensure_not_cancelled(cancel)?;
    tokio::fs::rename(&part, &destination).await?;
    *completed_bytes += downloaded;
    let sha256 = sha256_file(&destination).await?;
    Ok(FileRecord { path: file_name.to_string(), bytes: downloaded, sha256 })
}

async fn install_inner(app: &AppHandle, root: &Path, cancel: &AtomicBool) -> Result<LargeModelStatus> {
    tokio::fs::create_dir_all(root).await?;
    let client = reqwest::Client::builder()
        .user_agent("Video2CRT/0.1 model installer")
        .build()?;
    let mut completed_bytes = 0u64;
    let mut asr = Vec::new();
    for file_name in ASR_FILES {
        asr.push(download_file(app, &client, root, "asr", ASR_REPO, ASR_REVISION, file_name, cancel, &mut completed_bytes).await?);
    }
    let mut translation = Vec::new();
    for file_name in TRANSLATION_FILES {
        translation.push(download_file(app, &client, root, "translation", TRANSLATION_REPO, TRANSLATION_REVISION, file_name, cancel, &mut completed_bytes).await?);
    }
    ensure_not_cancelled(cancel)?;
    let manifest = Manifest { version: MODEL_VERSION, asr, translation };
    let manifest_tmp = root.join("manifest.json.part");
    let bytes = serde_json::to_vec_pretty(&manifest)?;
    tokio::fs::write(&manifest_tmp, bytes).await?;
    tokio::fs::rename(manifest_tmp, root.join("manifest.json")).await?;
    emit_progress(app, ModelInstallProgress {
        stage: "complete".to_string(), file: String::new(), progress: 1.0,
        downloaded_bytes: completed_bytes, total_bytes: ESTIMATED_BYTES,
        message: "高品質模型安裝完成".to_string(),
    });
    Ok(status_for_root(root))
}

#[tauri::command]
pub async fn get_large_model_status(app: AppHandle) -> Result<LargeModelStatus, String> {
    let root = model_root(&app).map_err(|e| e.to_string())?;
    Ok(status_for_root(&root))
}

#[tauri::command]
pub async fn install_large_model(app: AppHandle, state: State<'_, InstallState>) -> Result<LargeModelStatus, String> {
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut active = state.active.lock().map_err(|_| "模型安裝狀態鎖定失敗".to_string())?;
        if active.is_some() { return Err("模型安裝已在進行中".to_string()); }
        *active = Some(cancel.clone());
    }
    let result = async {
        let root = model_root(&app).map_err(|e| e.to_string())?;
        install_inner(&app, &root, &cancel).await.map_err(|e| e.to_string())
    }.await;
    if let Ok(mut active) = state.active.lock() { *active = None; }
    result
}

#[tauri::command]
pub fn cancel_large_model_install(state: State<'_, InstallState>) -> Result<(), String> {
    let active = state.active.lock().map_err(|_| "模型安裝狀態鎖定失敗".to_string())?;
    if let Some(cancel) = active.as_ref() { cancel.store(true, Ordering::Relaxed); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incomplete_manifest_is_not_ready() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("manifest.json"), br#"{"version":1,"asr":[],"translation":[]}"#).unwrap();
        let status = status_for_root(root.path());
        assert!(!status.available);
    }
}
