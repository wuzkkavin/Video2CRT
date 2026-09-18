# Video2CRT 系統分析（SA）

- Audience: SA、SD、RD、測試與維運
- Authority: current
- Status: 已驗證／部分 E2E 未驗證
- Last verified: 2026-09-18
- Evidence: `video2crt-app/src/App.tsx`、`src-tauri/src/orchestrator.rs`、`src-tauri/bin/pipeline_cli.py`

## 系統邊界

使用者在 Windows 桌面 UI 提供公開 URL 與選項；Tauri Rust 協調器管理本機 sidecar、ffmpeg、yt-dlp、模型與輸出檔。YouTube、Hugging Face、MiniMax 是外部邊界，只有明確流程需要時才連線。

## 主要角色

| 角色 | 行為 | 不應承擔的責任 |
| --- | --- | --- |
| 使用者 | 輸入 URL、選擇選項、確認模型與 API Key、檢查成品 | 不需管理 PATH、Python 或模型半成品 |
| UI | 收集選項、顯示狀態、阻擋重複啟動 | 不直接執行 ffmpeg 或保存 secret |
| Rust orchestrator | 建資料夾、呼叫工具、發布進度、處理取消與錯誤 | 不把字幕規則複製到 UI |
| Python sidecar | 音訊、ASR、字幕來源選擇、翻譯、SRT、字幕燒錄 | 不管理 UI 狀態或憑證 |
| 外部服務 | 提供影片、模型或雲端翻譯 | 不取得 Cookie 或使用者登入狀態 |

## 主流程狀態

`URL_INPUT → OPTIONS → MODEL_CONFIRM（必要時）→ DOWNLOAD → CROPDETECT → CRT_RENDER → ASR → SOURCE_CAPTION_SELECT → TRANSLATE（必要時）→ SUBTITLE_BURN → MUX → DONE`。

任何階段失敗都進入 `ERROR`，保留可追查的輸出與錯誤；取消進入 `CANCELLED`，不得把 `.part` 或不完整模型標成 ready。

## 關鍵分析規則

- 標題先由 oEmbed 取得，不能以 ID 資料夾替代，避免命名契約失效。
- yt-dlp 只有遇到明確 PO Token、Visitor Data 或 429 access gate 才有限回退 embedded client。
- YouTube 原語字幕須語言相符且時間完整，否則保留 ASR。
- 中文原文只輸出繁中單行；非中文雙語輸出原文與繁中兩行。
- 翻譯不完整時停止於 burn 前，不產生誤導性的雙語成品。

## 失敗與復原

下載失敗先檢查 URL／公開性與最後錯誤；裁切錯誤改用該影片手動 crop；ASR 對齊錯誤由 sidecar 無 word timestamps 重試；雲端翻譯失敗可改本機模式；模型下載失敗維持標準模型。禁止用 Cookie 或刪除舊輸出來「修復」。
