// Standalone repro: spawns yt-dlp with EXACTLY the same args the
// orchestrator uses (--no-playlist, --no-part, --js-runtimes node,
// --remote-components ejs:github). Confirms whether a given URL
// can be downloaded before the user touches the Tauri UI.
//
// Run: target/release/verify_download.exe <url>

fn main() -> anyhow::Result<()> {
    let url = std::env::args().nth(1).unwrap_or_else(|| {
        "https://www.youtube.com/watch?v=CaCSuzR4DwM".to_string()
    });
    let outdir = std::env::args().nth(2).unwrap_or_else(|| {
        r"C:\Users\asaialabs\Desktop\test_v2crt_e2e".to_string()
    });
    std::fs::create_dir_all(&outdir)?;
    let source_path = std::path::Path::new(&outdir).join("source.mp4");
    let _ = std::fs::remove_file(&source_path);

    println!("=== verify_download ===");
    println!("url:    {}", url);
    println!("outdir: {}", outdir);

    let yt_dlp = r"C:\Users\asaialabs\AppData\Local\hermes\hermes-agent\venv\Scripts\yt-dlp.exe";
    let localappdata = std::env::var("LOCALAPPDATA")
        .unwrap_or_else(|_| "C:/Users/asaialabs/AppData/Local".to_string());
    let node_exe = std::path::Path::new(&localappdata)
        .join("hermes").join("node").join("node.exe");

    let output_arg = format!("{}/source.%(ext)s", outdir);
    println!("-o: {}", output_arg);

    let mut cmd = std::process::Command::new(yt_dlp);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd.arg("-o").arg(&output_arg);
    cmd.arg("-f").arg("bv*[ext=mp4][height<=1080]+ba[ext=m4a]/b[ext=mp4]");
    cmd.arg("--merge-output-format").arg("mp4");
    cmd.arg("--no-part");
    cmd.arg("--no-playlist");
    cmd.arg("--js-runtimes").arg(format!("node:{}", node_exe.display()));
    cmd.arg("--remote-components").arg("ejs:github");
    cmd.arg(&url);
    cmd.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());

    println!("\nspawning...");
    let mut child = cmd.spawn()?;
    let status = child.wait()?;
    println!("exit: {:?}", status);
    if source_path.exists() {
        println!("source.mp4 size: {} bytes", std::fs::metadata(&source_path)?.len());
    } else {
        println!("FAIL: source.mp4 not created");
    }
    // drain stderr
    use std::io::Read;
    if let Some(mut stderr) = child.stderr.take() {
        let mut s = String::new();
        let _ = stderr.read_to_string(&mut s);
        if !s.is_empty() {
            println!("--- stderr ---\n{}", s);
        }
    }
    Ok(())
}
