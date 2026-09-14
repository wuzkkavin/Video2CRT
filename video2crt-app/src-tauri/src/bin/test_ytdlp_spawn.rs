// Standalone reproducer for the orchestrator's download command.
// Run with: target/release/test_ytdlp_spawn.exe <url>

use std::os::windows::process::CommandExt;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = std::env::args().nth(1).unwrap_or_else(|| {
        "https://www.youtube.com/watch?v=CaCSuzR4DwM".to_string()
    });
    let outdir = r"C:\Users\asaialabs\Desktop\test_v2crt3";
    let _ = std::fs::create_dir_all(outdir);
    // Clean any old source.mp4
    let source_path = std::path::Path::new(outdir).join("source.mp4");
    let _ = std::fs::remove_file(&source_path);

    let yt_dlp = r"C:\Users\asaialabs\AppData\Local\hermes\hermes-agent\venv\Scripts\yt-dlp.exe";
    let node = format!(
        "{}",
        std::path::Path::new(&std::env::var("LOCALAPPDATA")
            .unwrap_or_else(|_| "C:/Users/asaialabs/AppData/Local".to_string()))
            .join("hermes").join("node").join("node.exe")
            .to_string_lossy()
    );

    let mut cmd = Command::new(yt_dlp);
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    cmd.arg("-o").arg(format!("{}/source.%(ext)s", outdir));
    cmd.arg("-f").arg("bv*[ext=mp4][height<=1080]+ba[ext=m4a]/b[ext=mp4]");
    cmd.arg("--merge-output-format").arg("mp4");
    cmd.arg("--no-part");
    cmd.arg("--no-playlist");
    cmd.arg("--js-runtimes").arg(format!("node:{}", node));
    cmd.arg(&url);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd.spawn()?;
    let stderr = child.stderr.take().unwrap();
    let mut reader = BufReader::new(stderr).lines();
    while let Ok(Some(line)) = reader.next_line().await {
        println!("[stderr] {}", line);
    }
    let status = child.wait().await?;
    println!("[exit] {:?}", status);
    if source_path.exists() {
        println!("[OK] source.mp4 size = {} bytes", std::fs::metadata(&source_path)?.len());
    } else {
        println!("[FAIL] source.mp4 not created");
    }
    Ok(())
}
