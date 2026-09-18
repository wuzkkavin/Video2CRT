# Video2CRT Windows App

Video2CRT 是一個 Windows 桌面應用程式：貼上 YouTube URL，選擇字幕與翻譯方式，產生 CRT 顯像效果的 MP4。

## 使用者最短流程

1. 執行發布包 `dist-distributable/Video2CRT_0.1.0_x64-setup.exe`。
2. 安裝後開啟 Video2CRT，貼上公開 YouTube URL。
3. 選擇字幕模式與輸出資料夾，按「開始轉檔」。
4. 若尚未安裝高品質模型，選擇「安裝高品質模型」或直接使用內建標準模型。
5. 完成頁開啟輸出資料夾，確認 `final.mp4` 與 `zh-Hant.srt`。

主安裝包已包含 Python、ffmpeg、Node.js、yt-dlp、WebView2 與可離線執行的標準模型；使用者不必先在系統安裝這些工具。高品質模型是選配，需在應用程式內確認後下載。

## 文件入口

- [DEVELOPMENT_GUIDE.md](DEVELOPMENT_GUIDE.md)：架構、資料流與維護規則。
- [SPEC.md](SPEC.md)：需求範圍、使用者流程與可測驗收條件。
- [USER_GUIDE.md](USER_GUIDE.md)：一般使用者操作、模型安裝與錯誤處理。
- [TEST_ACCEPTANCE.md](TEST_ACCEPTANCE.md)：測試矩陣與本輪驗收證據。
- [RELEASE.md](RELEASE.md)：建置、發布包、hash 與可重現性界線。
- [AGENT_GUIDE.md](AGENT_GUIDE.md)：下一個 agent 的安全接手順序。
- [../HANDOFF.md](../HANDOFF.md)：跨 session 的最新狀態與下一步。

## 開發者快速檢查

在 `video2crt-app/` 執行：

```powershell
npm install
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

完整發布包建置需要 Windows、Python 3.12、Node.js/npm、Rust/Cargo 與網路下載建置資源：

```powershell
npm run tauri:build:distributable
```

不要把 API Key、Cookie、模型快取、影片輸出或 `.packaging/` 加入 Git。
