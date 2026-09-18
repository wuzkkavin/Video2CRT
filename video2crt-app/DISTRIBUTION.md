# Video2CRT Windows 發布包

正式建置由 `scripts/build-distributable.ps1` 完成。它會把 Python 3.12 embeddable runtime、faster-whisper／CTranslate2／SentencePiece／Hugging Face Hub、ffmpeg、Node.js、官方 yt-dlp、離線 WebView2、OpenCC 資源與釘選的離線 ASR／翻譯模型放進 Tauri runtime，然後產生 NSIS 安裝程式。

在具備 Python 3.12、Node.js/npm、Rust/Cargo 的 Windows 建置機執行：

```powershell
npm install
npm run tauri:build:distributable
```

輸出在 `dist-distributable/Video2CRT_0.1.0_x64-setup.exe`。使用者電腦不需要另外安裝 Python、ffmpeg、Node.js、yt-dlp 或標準模型；安裝程式採 current-user 安裝與 WebView2 offline installer。若要只驗證 runtime staging，可加上 `-SkipBuild`；`-SkipModels` 只適合預演，不是可離線發布包。

安裝包使用可在一般電腦工作的 CPU ASR fallback；有 NVIDIA 顯卡時字幕燒錄會先使用原本的 NVENC 設定，失敗才回退到 libx264。正式包的 SHA-256 以建置當次產出的檔案為準，請在交付前重新以 `Get-FileHash` 產生校驗值。

## 高品質模型

模型選擇只在 NSIS 安裝程序中出現。安裝檔複製完成後會詢問是否下載約 3.2 GB 的 pinned Whisper large-v3-turbo 與 m2m100 1.2B 模型；選「否」就只保留內建標準模型。高品質模型會寫入 `%LOCALAPPDATA%\Video2CRT\models\large`，以 manifest 驗證完成後才會被程式採用。

程式運行介面不會顯示模型選擇視窗，也不會在開始轉檔時臨時改變模型。若安裝時下載失敗，仍可直接使用內建標準模型，之後重新執行安裝程式即可重試。
