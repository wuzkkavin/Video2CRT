# Video2CRT 資料／DB 規格

- Audience: RD、資安、維運與接手 Agent
- Authority: current
- Status: 已驗證：本版本沒有伺服器資料庫；本文件記錄替代資料儲存
- Last verified: 2026-09-18
- Evidence: `settings.rs`、`model_manager.rs`、`orchestrator.rs`、`SPEC.md`

## DB 適用性

Video2CRT 0.1.0 是本機桌面工具，沒有 PostgreSQL、SQLite、雲端資料庫、帳號表或 migration。不要為了填 DB 文件而虛構 schema。若未來加入伺服器、同步或帳號，必須另立 schema、migration、備份與 rollback 規格。

## 本機資料契約

| 資料 | 儲存位置 | 內容／生命週期 |
| --- | --- | --- |
| API Key | Windows Credential Manager | service `Video2CRT`；UI 只讀已設定狀態 |
| 模型快取 | `%LOCALAPPDATA%\Video2CRT\models\large` | `.part` 完成 hash 與 manifest 後才啟用 |
| 工作輸出 | 桌面或使用者選取父資料夾 | source、raw、SRT、metadata、final；同名加序號 |
| runtime | 安裝目錄 | 內嵌 Python、ffmpeg、yt-dlp、標準模型，不作使用者資料庫 |
| 暫存檔 | 工作輸出或 `.packaging` | 只供該次流程與建置，禁止進 Git |

## 格式與完整性

字幕 JSON／SRT 為 UTF-8；SRT 時間不可倒退或重疊。模型 manifest 保存檔名、大小與 SHA-256；缺檔、尺寸不符或 hash 不符視為未安裝。輸出資料夾不得覆寫既有成果。

## 備份、刪除與未來 migration

使用者自行保留 `final.mp4` 與必要字幕；程式不自動清空既有成果。任何未來資料結構變更需新增 version、migration、rollback 與舊版相容性測試，並更新 `11_DECISIONS_TRACEABILITY.md`。
