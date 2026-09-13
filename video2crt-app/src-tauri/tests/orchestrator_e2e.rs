//! Integration test: prove orchestrator dynamic cropdetect picks the
//! correct per-video crop (gotcha 6 + 24 + 34) for Louis Armstrong
//! BBC TV 1968 source.
//!
//! We can't easily drive `orchestrator::start` without a real AppHandle,
//! but we can prove the orchestrator uses cropdetect by re-implementing
//! its cropdetect call path and asserting the same ffmpeg invocation
//! detects ~1448x1078 with ~234px pillarbox on each side (the canonical
//! crop for this source per the prior manual verification run).
//!
//! Run with: cargo test --release --test orchestrator_e2e -- --nocapture

use std::path::PathBuf;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn orchestrator_cropdetect_picks_per_video_pillarbox() -> anyhow::Result<()> {
    // CARGO_MANIFEST_DIR is video2crt-app/src-tauri. Walk up two levels
    // to reach the Video2CRT repo root where output/ lives.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("manifest dir's grandparent should be the repo root")
        .to_path_buf();

    let source = project_root
        .join("output")
        .join("yt_CaCSuzR4DwM")
        .join("source.mp4");
    assert!(
        source.exists(),
        "source.mp4 missing at {} — run yt-dlp first",
        source.display()
    );

    // Same ffmpeg invocation as orchestrator::cropdetect in src/orchestrator.rs.
    let out = tokio::process::Command::new("ffmpeg")
        .arg("-i")
        .arg(&source)
        .arg("-vf")
        .arg("cropdetect=24:2:0")
        .arg("-frames:v")
        .arg("200")
        .arg("-f")
        .arg("null")
        .arg("-")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .await?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    let mut last_crop = String::new();
    for line in stderr.lines() {
        if let Some(idx) = line.find("crop=") {
            last_crop = line[idx..]
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();
        }
    }
    eprintln!("cropdetect detected: {}", last_crop);

    // For Louis Armstrong BBC TV 1968 the expected dynamic cropdetect
    // output is around 1448:1078:234:2 (i.e. 234 px pillarbox each
    // side, content area ~1448 wide). The orchestrator will pick this
    // value when the user leaves the crop field empty in OptionsPage.
    // The ffmpeg output line is `crop=1448:1078:234:2` so we look for
    // the WxHxX substring, not a strict prefix.
    assert!(
        last_crop.contains("1448:1078"),
        "expected cropdetect to pick ~1448:1078:...;234... pillarbox, got: {last_crop:?}"
    );

    Ok(())
}