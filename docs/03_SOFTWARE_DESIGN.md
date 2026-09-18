# Video2CRT 軟體設計（SD）

- Audience: RD、維護者、測試與接手 Agent
- Authority: current
- Status: 已驗證（模組與路徑已對照原始碼）
- Last verified: 2026-09-18
- Evidence: `src/App.tsx`、`src-tauri/src/lib.rs`、`orchestrator.rs`、`pipeline_cli.py`

## 模組責任

| 模組 | 來源 | 責任與邊界 |
| --- | --- | --- |
| React UI | `src/App.tsx`, `src/pages/` | URL、選項、頁面狀態、事件與結果，不持有 secret |
| Tauri commands | `src-tauri/src/lib.rs` | UI IPC、事件註冊、runtime 路徑解析 |
| Orchestrator | `src-tauri/src/orchestrator.rs` | 工作佇列、下載、crop、CRT、sidecar、取消、輸出命名 |
| Model manager | `src-tauri/src/model_manager.rs` | pinned repo/revision、`.part`、hash、manifest、進度與取消 |
| Settings | `src-tauri/src/settings.rs` | Windows Credential Manager 的 API Key 存取 |
| Python pipeline | `src-tauri/bin/pipeline_cli.py` | 16 kHz 音訊、ASR、字幕、翻譯、burn、封裝 |
| Subtitle engine | `src-tauri/bin/subtitle_engine.py` | VTT/SRT 正規化、語言、時間軸、OpenCC、翻譯清理 |

## 資料流與檔案契約

`source.mp4 → raw.mp4（CRT、無聲）→ subtitled.mp4（字幕、無聲）→ final.mp4（原始音訊封裝）`。字幕資料依序包含 `faster_whisper_out.json`、`subtitle_segments.json`、`subtitle_translations.json`、`zh-Hant.srt`。每次工作使用新標題資料夾，不覆寫既有成果。

## Design IDs

- `DES-001`：self-contained runtime 與 current-user 安裝。
- `DES-002`：URL → options → progress → done 的 UI wizard。
- `DES-003`：以影片標題與序號建立不覆寫輸出資料夾。
- `DES-004`：Rust progress events、取消與錯誤狀態。
- `DES-005`：Python subtitle engine 的語言、時間軸與翻譯契約。
- `DES-006`：高品質模型的確認、下載、hash、manifest 與切換生命週期。
- `DES-007`：翻譯不完整時在 burn 前阻擋。
- `DES-008`：Credential Manager secret boundary。
- `DES-009`：source/raw/subtitled/final 影音格式契約。
- `DES-010`：發布建置、安裝冒煙與 SHA-256 證據。

## IPC 與事件

UI 透過 Tauri commands 啟動／取消工作、查詢模型狀態與保存設定；Rust 以 progress event 回傳 stage、百分比、訊息與完成結果。高品質模型下載使用 `model://progress`，只有 manifest 與 hash 完整才回報 ready。

## 併發與錯誤

開始按鈕在工作期間鎖定；Rust 以 Tokio 管理非同步流程與取消 token。外部工具非零退出、無效字幕時間、翻譯缺漏、模型 hash 不符都應中止相應階段，保留 log／中間檔供診斷，不偽造完成。

## 相容性不變量

CRT shader、crop 策略、1920×1080、字幕燒錄的 `h264_nvenc/p4/cq 23` 與原始音訊封裝是既有視覺基準。修字幕或下載不得未經需求修改這些參數。
