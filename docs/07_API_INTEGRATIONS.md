# Video2CRT API 與外部整合規格

- Audience: RD、資安、測試與維運
- Authority: current
- Status: 已驗證／服務端行為可能漂移
- Last verified: 2026-09-18
- Evidence: `orchestrator.rs`、`pipeline_cli.py`、`translator.rs`、`model_manager.rs`

## 整合清單

| 整合 | 用途 | 認證／資料邊界 | 回退與限制 |
| --- | --- | --- | --- |
| YouTube oEmbed | 取得公開影片標題 | 公開 URL；不使用 Cookie | 失敗即停止，不改用 ID 命名 |
| yt-dlp | 下載公開影音／字幕 | 公開網路請求；不自動登入 | 只對明確 access gate 回退 embedded 一次 |
| ffmpeg／ffprobe | 影音、crop、CRT、字幕、封裝 | 本機執行 | 非零退出保留錯誤，不偽造完成 |
| Hugging Face | 高品質模型檔案 | 固定 repo/revision，安裝程序確認後下載 | manifest；失敗維持標準模型 |
| MiniMax | 選用雲端翻譯 | 使用者主動選擇且已保存 Key 才送字幕文字 | 本機優先失敗才 fallback；不傳影片 |
| Tauri IPC | UI 與 Rust 協調器 | 本機 command/event | 進度、取消、錯誤與結果需有明確狀態 |

## API Key 與網路規則

MiniMax Key 只從 Credential Manager 讀取；不得出現在 URL、log、SRT、JSON 或 Markdown。雲端翻譯只傳必要字幕文字；ASR、CRT、封裝與本機翻譯留在本機。

## 版本與失敗處理

模型 revision 與 hash 必須寫入程式／manifest；外部服務錯誤要記錄非敏感錯誤與 stage。YouTube 服務端規則可能變動，測試通過只代表當輪證據，不代表永久相容。
