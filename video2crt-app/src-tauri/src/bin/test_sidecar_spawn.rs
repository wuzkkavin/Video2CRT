// test_sidecar_spawn: spawns Python sidecar exactly like
// orchestrator.rs's run_sidecar() does, with lossy UTF-8 line reads.
// Verifies that the bytes-based reader doesn't raise InvalidData even
// if the pipe contains stray cp1252 lead bytes between PROGRESS lines.
//
// Run: target/release/test_sidecar_spawn.exe

use std::os::windows::process::CommandExt;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let python_bin = r"C:\Users\asaialabs\AppData\Local\hermes\hermes-agent\venv\Scripts\python.exe";
    let py_script = r"C:\Users\asaialabs\Documents\Hermes\Video2CRT\video2crt-app\src-tauri\bin\pipeline_cli.py";
    let payload = r#"{"url": "", "outputDir": "C:\\Users\\asaialabs\\Documents\\Hermes\\Video2CRT\\output\\yt_CaCSuzR4DwM", "videoId": "CaCSuzR4DwM", "crop": "1448:1078:234:2", "asrLanguage": null, "cloudTranslation": false, "translationModel": null}"#;
    let mut cmd = Command::new(python_bin);
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    cmd.env("PYTHONPATH", r"C:\Users\asaialabs\AppData\Local\hermes\hermes-agent\venv\Lib\site-packages");
    let mut child = cmd
        .arg(py_script)
        .arg(payload)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("no stdout"))?;
    let mut reader = BufReader::new(stdout);
    let mut buf: Vec<u8> = Vec::with_capacity(512);
    let mut count = 0;
    loop {
        buf.clear();
        let n = reader.read_until(b'\n', &mut buf).await?;
        if n == 0 { break; }
        count += 1;
        let line = String::from_utf8_lossy(&buf).trim_end_matches("\r\n").to_string();
        let trimmed = line.trim();
        println!("[{:03}] {}", count, &trimmed[..trimmed.len().min(120)]);
    }
    let status = child.wait().await?;
    println!("child exit: {:?}", status);
    Ok(())
}
