//! Standalone end-to-end pipeline runner.
//!
//! Drives the same 6-stage pipeline the Tauri orchestrator runs, but
//! without the Tauri runtime, so we can verify that the .exe actually
//! works end-to-end before shipping. The implementation mirrors
//! `orchestrator.rs` stage-by-stage; if they ever drift, update both.
//!
//! Usage:
//!     target/release/run_e2e.exe <youtube-url> [output_dir]


#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "https://www.youtube.com/watch?v=CaCSuzR4DwM".to_string());
    let output_dir: PathBuf = std::env::args()
        .nth(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Users\asaialabs\Desktop\run_e2e"));
    std::fs::create_dir_all(&output_dir)?;

    println!("=== run_e2e ===");
    println!("url:        {}", url);
    println!("output_dir: {}", output_dir.display());

    let localappdata = std::env::var("LOCALAPPDATA")
        .unwrap_or_else(|_| "C:/Users/asaialabs/AppData/Local".to_string());

    // ---------- Stage 1: yt-dlp download ----------
    println!("\n--- Stage 1: yt-dlp download ---");
    let yt_dlp = localappdata.replace("\\", "/") + "/hermes-agent/venv/Scripts/yt-dlp.exe";
    let node_exe = localappdata.replace("\\", "/") + "/hermes/node/node.exe";
    let yt_dlp_path = Path::new(&yt_dlp);
    if !yt_dlp_path.exists() {
        anyhow::bail!("yt-dlp not found at {}", yt_dlp);
    }
    let output_arg = format!("{}/source.%(ext)s", output_dir.display());
    let download_status = std::process::Command::new(yt_dlp_path)
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .arg("-o").arg(&output_arg)
        .arg("-f").arg("bv*[ext=mp4][height<=1080]+ba[ext=m4a]/b[ext=mp4]")
        .arg("--merge-output-format").arg("mp4")
        .arg("--no-part")
        .arg("--no-playlist")
        .arg("--js-runtimes").arg(format!("node:{}", node_exe))
        .arg("--remote-components").arg("ejs:github")
        .arg(&url)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .status()?;
    if !download_status.success() {
        anyhow::bail!("yt-dlp failed with {:?}", download_status);
    }
    let source_mp4 = output_dir.join("source.mp4");
    if !source_mp4.exists() {
        anyhow::bail!("source.mp4 not produced");
    }
    println!("source.mp4: {} bytes", std::fs::metadata(&source_mp4)?.len());

    // ---------- Stage 2: cropdetect ----------
    println!("\n--- Stage 2: cropdetect ---");
    let cropdetect_out = std::process::Command::new("ffmpeg")
        .creation_flags(0x0800_0000)
        .arg("-i").arg(&source_mp4)
        .arg("-vf").arg("cropdetect")
        .arg("-frames:v").arg("30")
        .arg("-f").arg("null").arg("-")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()?;
    let cropdetect_stderr = String::from_utf8_lossy(&cropdetect_out.stderr);
    let crop = cropdetect_stderr
        .lines()
        .filter(|l| l.contains("crop="))
        .filter_map(|l| {
            let idx = l.find("crop=")?;
            let rest = &l[idx + 5..];
            let end = rest.find(|c: char| !c.is_ascii_digit() && c != ':')?;
            Some(rest[..end].to_string())
        })
        .last()
        .ok_or_else(|| anyhow::anyhow!("cropdetect produced no crop= line"))?;
    println!("detected crop: {}", crop);

    // ---------- Stage 3: CRT render (libplacebo + h264_nvenc) ----------
    println!("\n--- Stage 3: CRT render ---");
    let shader_src = Path::new("crt.glsl");
    if !shader_src.exists() {
        // Try Desktop root as a fallback.
        let desktop_shader = Path::new(r"C:\Users\asaialabs\Desktop\crt.glsl");
        if desktop_shader.exists() {
            std::fs::copy(desktop_shader, output_dir.join("crt.glsl"))?;
        } else {
            anyhow::bail!("crt.glsl shader not found");
        }
    } else {
        std::fs::copy(shader_src, output_dir.join("crt.glsl"))?;
    }
    // Get input resolution.
    let ffprobe_out = std::process::Command::new("ffprobe")
        .creation_flags(0x0800_0000)
        .arg("-v").arg("error")
        .arg("-select_streams").arg("v:0")
        .arg("-show_entries").arg("stream=width,height")
        .arg("-of").arg("csv=p=0")
        .arg(&source_mp4)
        .output()?;
    let wh = String::from_utf8_lossy(&ffprobe_out.stdout);
    let mut parts = wh.trim().split(',');
    let w: u32 = parts.next().unwrap_or("1920").parse().unwrap_or(1920);
    let h: u32 = parts.next().unwrap_or("1080").parse().unwrap_or(1080);
    println!("resolution: {}x{}", w, h);
    let render_status = std::process::Command::new("ffmpeg")
        .creation_flags(0x0800_0000)
        .current_dir(&output_dir)
        .arg("-y").arg("-hwaccel").arg("auto")
        .arg("-i").arg(&source_mp4)
        .arg("-vf").arg(format!(
            "crop={},libplacebo=custom_shader_path=crt.glsl:w={}:h={}:fps=30:force_original_aspect_ratio=0",
            crop, w, h
        ))
        .arg("-c:v").arg("h264_nvenc")
        .arg("-rc:v").arg("vbr")
        .arg("-cq:v").arg("18")
        .arg("-preset:v").arg("p4")
        .arg("-pix_fmt").arg("yuv420p")
        .arg("raw.mp4")
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .status()?;
    if !render_status.success() {
        anyhow::bail!("CRT render failed");
    }
    let raw_mp4 = output_dir.join("raw.mp4");
    if !raw_mp4.exists() {
        anyhow::bail!("raw.mp4 not produced");
    }
    println!("raw.mp4: {} bytes", std::fs::metadata(&raw_mp4)?.len());

    // ---------- Stage 4-6: Python sidecar ----------
    println!("\n--- Stage 4-6: Python sidecar ---");
    let python_exe = localappdata.replace("\\", "/") + "/hermes-agent/venv/Scripts/python.exe";
    let sidecar_script = std::env::current_dir()
        .map_err(|e| anyhow::anyhow!("cwd err: {}", e))?
        .join("video2crt-app/src-tauri/bin/pipeline_cli.py");
    if !sidecar_script.exists() {
        anyhow::bail!("sidecar script not found at {}", sidecar_script.display());
    }
    let payload = serde_json::json!({
        "url": url,
        "outputDir": output_dir.to_string_lossy(),
        "videoId": "run_e2e",
        "crop": crop,
        "asrLanguage": null,
        "cloudTranslation": false,
        "translationModel": "",
    });
    let payload_str = serde_json::to_string(&payload)?;
    let sidecar_status = std::process::Command::new(&python_exe)
        .creation_flags(0x0800_0000)
        .env("PYTHONPATH", format!("{}/hermes-agent/venv/Lib/site-packages", localappdata.replace("\\", "/")))
        .arg(&sidecar_script)
        .arg(&payload_str)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    if !sidecar_status.success() {
        anyhow::bail!("Python sidecar exited with {:?}", sidecar_status);
    }

    let final_mp4 = output_dir.join("final.mp4");
    if !final_mp4.exists() {
        anyhow::bail!("final.mp4 not produced");
    }
    let srt = output_dir.join("zh-Hant.srt");
    let n_entries = if srt.exists() {
        std::fs::read_to_string(&srt)?
            .split("\n\n")
            .filter(|s| !s.trim().is_empty())
            .count()
    } else { 0 };

    println!("\n=== DONE ===");
    println!("final.mp4: {} bytes", std::fs::metadata(&final_mp4)?.len());
    println!("zh-Hant.srt: {} entries", n_entries);
    Ok(())
}
