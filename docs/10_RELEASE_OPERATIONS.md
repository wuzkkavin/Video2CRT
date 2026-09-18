# Video2CRT 發布與維運規格

- Audience: 發布、維運、RD 與接手 Agent
- Authority: current
- Status: 已驗證；乾淨帳戶完整流程未驗證
- Last verified: 2026-09-18
- Evidence: `video2crt-app/RELEASE.md`、`build-distributable.ps1`、安裝包與 silent install

## 產物與建置

- 版本：0.1.0，Windows x64，Tauri 2 NSIS current-user。
- 建置：`npm install`、`npm run tauri:build:distributable`。
- 內容：Python 3.12 embeddable、ffmpeg、Node.js、yt-dlp、離線 WebView2、OpenCC、標準 ASR／翻譯模型與 app 資源。
- 產物：`video2crt-app/dist-distributable/Video2CRT_0.1.0_x64-setup.exe`。
- 已知 hash：`96AAEF70137210371A2E471727F7E4FDE9CECE3DA02B0047A5228265B8B66232`；每次重建需重新計算。

## 發布核對

先跑 gotcha／依賴、根目錄 13 項、字幕 26 項、Rust 12 項、前端 build，再執行發布腳本。安裝後確認 `video2crt.exe`、embedded Python、ffmpeg、yt-dlp、標準模型存在。silent install 已通過；乾淨帳戶與完整 GUI E2E 仍未驗證。

## 回退與維運

版本發布前保留上一個安裝包與 hash；若安裝或啟動失敗，回退上一個已驗證版本，不刪除使用者輸出。模型下載失敗維持標準模型。若外部 YouTube／Hugging Face／MiniMax 行為改變，先建立可重現證據與窄測試再修改。

## Git 與交接

只 stage 指定原始碼、文件、腳本與 lockfile；父層 `output/`、模型快取、log、影片與憑證不提交。push 後比較本地 `HEAD` 與遠端分支；根目錄 `HANDOFF.md` 只記錄狀態與下一步。
