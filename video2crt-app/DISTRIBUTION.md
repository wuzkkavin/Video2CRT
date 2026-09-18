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

第一次開始字幕轉檔時，如果尚未安裝高品質模型，應用程式會先顯示選擇畫面。使用者可以直接使用內建標準模型，或確認下載約 3.2 GB 的 pinned Whisper large-v3-turbo 與 m2m100 1.2B 模型。下載會寫入目前 Windows 使用者的 `%LOCALAPPDATA%\Video2CRT\models\large`，以暫存檔完成下載、計算 SHA-256 並寫入 manifest 後才啟用；取消或失敗不會覆蓋內建模型。

高品質模型是可選的，因為把它與主安裝包合併會超過目前 NSIS 單檔大小限制。若要完全離線部署高品質模型，應另行提供模型包並在離線環境匯入。
