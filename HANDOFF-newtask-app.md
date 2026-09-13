# Handoff — Video2CRT Windows APP (Tauri 2)

## 任務
把 `C:\Users\asaialabs\Documents\Hermes\Video2CRT\` 的 CLI pipeline 做成 Windows .exe APP。
輸入 YouTube URL + 一鍵轉檔，復用現有 pipeline：yt-dlp → cropdetect → ffmpeg + libplacebo CRT shader → 本地 faster-whisper ASR → SRT 燒錄 → mux。

## 技術棧（已選定）
- **Tauri 2 + React + TypeScript**（WebView2，Win11 已內建）
- **後端 Rust**：`std::process::Command` 或 `tokio::process` 呼叫 ffmpeg / yt-dlp
- **ASR**：本地 faster-whisper（Python sidecar 或 Python embed）
- **打包**：portable `.exe`（不要 .msi）

## 功能需求
1. **單檔模式**（不支援批次）
2. **UI 流程**：URL 輸入 → 選項 → 進度條（階段顯示）→ 完成頁 + 「在 Explorer 開啟」按鈕（等同 MEDIA: link 行為）
3. **完整復用現有 pipeline** + **所有 34 條 gotcha 規則帶進 app**

### 進度階段（建議拆分）
- 階段 1: yt-dlp 下載（10%）
- 階段 2: cropdetect 黑邊偵測（5%）
- 階段 3: libplacebo CRT shader encode（50%，最慢）
- 階段 4: faster-whisper ASR（20%，CPU 慢）
- 階段 5: SRT 燒錄（10%）
- 階段 6: mux 完成（5%）

### Crop 預設值（gotcha 6 + 24 + 34）— **2026-09-13 改為 dynamic cropdetect**
- ~~4:3 內容在 16:9 容器內 → `crop=960:720:160:0`~~ ← 錯!這個 hardcode 假設是 Bobby Brown 等特定影片,對 Louis Armstrong BBC TV (內容 1448 wide) 會切掉 488 px 真實內容
- **新行為**:OptionsPage 預設 crop 欄位**留空**,Rust orchestrator 自動跑 `ffmpeg cropdetect=24:2:0` 偵測 pillarbox,然後用偵測到的 `crop=W:H:X:Y`(例如 `1448:1078:234:2`)進 libplacebo
- gotcha 34: cropdetect 是 luminance-based,若 source 有彩色 dotmask/stripes 可能誤判。User 可在 OptionsPage 手動覆寫 crop 值
- gotcha 24 trade-off:即使偵測正確,4:3 內容 stretch 到 1920x1080 仍有水平變形(accept 或在 OptionsPage 改用其他選項)

## API Key 管理
- **選填，預設關閉**
- **Provider**: Minimax International（base URL `https://api.minimax.io/v1`，Bearer token 格式）
- **儲存**: Windows Credential Manager（Rust crate `keyring` 或 `tauri-plugin-stronghold`）
- **驗證**: `GET /v1/models` 帶 Bearer token → 載入真實模型清單作為下拉選單；失敗 fallback 用硬編碼清單
- **用途**：唯一用途是「雲端字幕翻譯」（覆蓋本地 ASR 後的翻譯邏輯）。**影片轉檔本身不需要 key**

### Minimax 硬編碼模型清單（fallback）
**語言模型（用於字幕翻譯）**：
- `MiniMax-M3` ← 最新，1M context，**預設**
- `MiniMax-M2.7` / `M2.7-highspeed`
- `MiniMax-M2.5` / `M2.5-highspeed`（legacy）
- `MiniMax-M2.1` / `M2.1-highspeed`（legacy）
- `MiniMax-M2`（legacy）

**視訊模型**（本 app 不用但列出）：
- `MiniMax H3` / `H3 Max`
- Legacy: `Hailuo 2.3` / `2.3Fast` / `02`

**語音模型**（TTS，本 app 不需要）：
- `speech-2.8-hd` / `speech-2.8-turbo`
- Legacy: `2.6-hd` / `2.6-turbo` / `02-hd` / `02-turbo`

## 輸出路徑
`C:\Users\asaialabs\Documents\Hermes\Video2CRT\output\yt_<video-id>\`
（已存在，每個影片一個資料夾，裡面放 source.mp4, raw.mp4, final.mp4, zh-Hant.srt, handoff.md 等）

## 不要做
- ❌ 自動更新
- ❌ 批次轉檔
- ❌ 雲端 ASR（Minimax 沒 ASR API，無解）
- ❌ 上傳使用者私人資料到任何外部服務（除非使用者主動填 key 並同意雲端翻譯）

## 開始前必做（pre-flight）
1. `cd C:\Users\asaialabs\Documents\Hermes\Video2CRT\` 確認 git 狀態
2. 列出既有 `output/` 內容（已有 11+ 個完成資料夾，例如 `Top_100_Party_Hits/`, `Amuro_Namie_DANCING_JUNK/`, `初恋_回春丹_MV/`, `Bobby_Brown_Every_Little/` 等）
3. 檢查 `scripts/install_skill.py`（確認 skill v34+ ALL PASS）
4. **必載入 skill**: `video-crt-geom-libplacebo`（用 `skill_view`）

## 建 todo list 建議
分階段：
1. Scaffold Tauri 2 專案（cargo create-tauri-app + React TS 模板）
2. 最小轉檔（CRT only）— 證明 ffmpeg + libplacebo sidecar 跑得起來
3. 加 yt-dlp + cropdetect + crop
4. 加 faster-whisper ASR（Python embed 或 sidecar）
5. 加 SRT 燒錄 + mux
6. 加 API key 管理（Windows Credential Manager + `/v1/models` 動態載入）
7. 加雲端翻譯（選填，預設關閉）
8. 視覺驗證 gotcha 10（多時間點抽幀）
9. 打包 `.exe`（tauri build）
10. handoff.md + 寫成 skill 並 push 到 HermesFullSetup（gotcha 27）

## Git 規範
- Video2CRT repo：本次任務會**新建** `video2crt-app/` 或類似子目錄裝 Tauri 專案
- HermesFullSetup repo：如有 gotcha 新增或 skill 變更，按 gotcha 27 stage only 新檔，commit message 用 `feat(skills): <trigger>` 前綴
- **不要** commit `.env` / dirty `config.yaml` / 自動 sync 產物

## 對話結尾規範
- 主動問使用者要不要把這次學到的東西寫成 skill 並 push 到 HermesFullSetup（按 gotcha 27 規則）
- 按 gotcha 30 對最終交付的影片輸出 `MEDIA:` link
- 按 gotcha 33 對 Tauri .exe 產出也給 `MEDIA:` link

## 重要警告（給接手的我）
- **不要在 chat text 寫「立即開工」就停下** — 必須真的呼叫 `todo_list` 或其他 tool 才能算「開始」
- 若對話卡住無回應，誠實告訴使用者原因（tool call 沒發出 / context 爆 / 格式錯誤），不要假裝在做事
- 對話過 30 回合主動提醒使用者考慮換新對話
