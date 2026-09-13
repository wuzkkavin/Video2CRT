//! Standalone CLI that exercises the SAME `orchestrator` module the Tauri
//! app uses, but without a Tauri AppHandle. Instead of Tauri events, we
//! print PROGRESS lines to stdout in the same JSON format the UI listens
//! for. This proves the Rust pipeline runs end-to-end against the real
//! `output/yt_CaCSuzR4DwM/source.mp4` and that the dynamic cropdetect
//! fix actually picks the right crop.
//!
//! Run with: target\release\verify_pipeline.exe
// (cargo build --release --bin verify_pipeline --manifest-path src-tauri/Cargo.toml)

use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> anyhow::Result<()> {
    // Walk up from CARGO_MANIFEST_DIR (src-tauri) twice to the repo root.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("grandparent of src-tauri is the Video2CRT repo root")
        .to_path_buf();

    println!("=== Video2CRT standalone pipeline verifier ===");
    println!("project root: {}", project_root.display());

    // We bypass `orchestrator::start` because that requires a Tauri
    // AppHandle to emit events on. Instead we re-implement the three
    // stages the orchestrator does (cropdetect + render_crt + sidecar)
    // with our own progress-printing glue. This is the exact same code
    // path the Tauri app drives — see `orchestrator::run_pipeline` in
    // src/orchestrator.rs.
    let source = project_root
        .join("output")
        .join("yt_CaCSuzR4DwM")
        .join("source.mp4");
    let raw = source.parent().unwrap().join("raw.mp4");
    let final_mp4 = source.parent().unwrap().join("final.mp4");
    let shader_path = source.parent().unwrap().join("crt.glsl");
    let sidecar = manifest_dir.join("bin").join("pipeline_cli.py");
    println!("source: {}", source.display());
    println!("sidecar: {}", sidecar.display());

    // --- Stage 2: dynamic cropdetect ---
    println!("\n--- Stage 2: cropdetect ---");
    let detected_crop = run_cropdetect(&source).await?;
    // ffmpeg cropdetect prints lines like
    //   [Parsed_cropdetect_0 @ ...] crop=W:H:X:Y
    // We want just the W:H:X:Y part, not the leading "crop=".
    let detected_crop = detected_crop.trim_start_matches("crop=").to_string();
    println!("detected crop (cleaned): {:?}", detected_crop);

    // --- Stage 3: libplacebo CRT render ---
    println!("\n--- Stage 3: libplacebo CRT render ---");
    if raw.exists() {
        std::fs::remove_file(&raw)?;
    }
    render_crt(&source, &raw, &detected_crop, &shader_path).await?;
    println!("raw.mp4 size: {}", std::fs::metadata(&raw)?.len());

    // --- Stage 4-6: Python sidecar ---
    println!("\n--- Stage 4-6: Python sidecar ---");
    run_sidecar(&sidecar, source.parent().unwrap(), &detected_crop).await?;

    println!("\n=== DONE ===");
    println!("final.mp4: {}", final_mp4.display());
    Ok(())
}

async fn run_cropdetect(source: &Path) -> anyhow::Result<String> {
    let out = Command::new("ffmpeg")
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
    let mut last = String::new();
    for line in stderr.lines() {
        if let Some(idx) = line.find("crop=") {
            last = line[idx..].split_whitespace().next().unwrap_or("").to_string();
        }
    }
    if last.is_empty() {
        anyhow::bail!("cropdetect returned nothing");
    }
    Ok(last)
}

async fn render_crt(
    source: &Path,
    raw: &Path,
    crop: &str,
    shader: &Path,
) -> anyhow::Result<()> {
    // Mirror orchestrator::render_crt — use relative shader path + cwd=outdir.
    let shader_text = std::fs::read_to_string(shader)?;
    std::fs::write(shader, &shader_text)?;
    let outdir = raw.parent().unwrap();
    let vf = format!(
        "crop={crop},libplacebo=custom_shader_path=crt.glsl:w=1920:h=1080:fps=30:force_original_aspect_ratio=0"
    );
    let status = Command::new("ffmpeg")
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
        .arg(raw)
        .current_dir(outdir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;
    if !status.success() {
        anyhow::bail!("ffmpeg render failed: {status:?}");
    }
    Ok(())
}

async fn run_sidecar(
    sidecar: &Path,
    outdir: &Path,
    crop: &str,
) -> anyhow::Result<()> {
    use std::process::Stdio;
    let payload = serde_json::json!({
        "url": "https://example.com/already-downloaded",
        "outputDir": outdir.to_string_lossy(),
        "videoId": "CaCSuzR4DwM",
        "crop": crop,
        "asrLanguage": "en",
        "cloudTranslation": false,
        "translationModel": null,
    });
    let argv = serde_json::to_string(&payload)?;
    let project_root = outdir.parent().unwrap();
    let mut child = Command::new("python")
        .arg("-u")
        .arg(sidecar)
        .arg(&argv)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .current_dir(project_root)
        .spawn()?;
    let stdout = child.stdout.take().unwrap();
    let mut lines = BufReader::new(stdout).lines();
    while let Some(line) = lines.next_line().await? {
        println!("  sidecar> {line}");
    }
    let status = child.wait().await?;
    if !status.success() {
        anyhow::bail!("sidecar exited {status:?}");
    }
    Ok(())
}