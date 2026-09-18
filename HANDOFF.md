# Video2CRT - HANDOFF

## 最終收工紀錄 — 2026-09-18 運行介面回歸修正

### 本輪完成

- 移除運行時的 `ModelInstallDialog`、模型選擇 IPC 與相關前端狀態；主程式不再顯示「使用標準模型」等選項。
- 保留自攜式 runtime、標準模型與 CRT shader 路徑；安裝器以 `installer-hooks.nsh` 在檔案複製後詢問是否下載高品質模型。
- 新增 `scripts/install-large-model.py`，由 bundled Python 將固定 revision 模型下載至使用者資料夾，完成 manifest 後才由 sidecar 靜默採用。
- 更新發布、使用、SA/SD/UI/UX/RD/DB/API/QA 文件，明確區分「安裝時選擇」與「程式運行介面」。

### 驗證

- `npm run build`：通過。
- `cargo check --manifest-path src-tauri/Cargo.toml`：通過。
- `cargo test --manifest-path src-tauri/Cargo.toml --all-targets -- --test-threads=1`：11 個 library 測試與 1 個 integration 測試通過；binary harness 為 0 tests 但已編譯。
- Python `py_compile`：`install-large-model.py`、`pipeline_cli.py`、`subtitle_engine.py` 通過。
- `npm run tauri:build`：NSIS x64 安裝器建置通過；產物已複製至 `video2crt-app/dist-distributable/`。
- 安裝器 SHA-256：`BA47C87C444C0A2E8ACE92C862F740E2421C8390B7E042B080592D86948625A0`。
- 產物內已確認 `install-large-model.py` 與 installer hook；前端搜尋不再有模型對話框或「使用標準模型」字串。

### Git 狀態

- `0b508b8 fix(app): move model choice to installer` 已提交並推送至 `origin/main`。
- 本輪只提交程式、安裝器與文件；既有輸出、歌詞、暫存編譯檔與其他未追蹤資料保持原樣。

### 未驗證與風險

- 尚未在乾淨 Windows 使用者帳戶完成互動式安裝與 GUI 到 `final.mp4` 的完整視覺 E2E。
- 尚未實際下載完整約 3.2 GB 高品質模型；安裝器的選擇、失敗回退與 manifest 路徑已完成結構／建置驗證。
- YouTube、Hugging Face 與 WebView2 外部服務行為仍可能變動；不可把本輪建置通過解讀成外部服務永久可用。

### 下一個安全動作

先在隔離 Windows 帳戶以 `dist-distributable/Video2CRT_0.1.0_x64-setup.exe` 測試「略過模型」與「下載模型」兩條安裝路徑，再做一次真實影片 GUI E2E；不要恢復運行時模型選擇視窗。

## 最終收工紀錄 — 2026-09-18 完全開發設計指南

### 本輪完成

- 建立全域 skill **完全開發設計指南**，machine id 為 `complete-development-design-guide`；一次涵蓋需求、SA、SD、UI/UX、RD、資料／DB、API、QA、資安、發布、決策與交接文件。
- 為 Video2CRT 建立 12 份角色化 Markdown 文件與總索引 `docs/PROJECT_DOCUMENTATION_INDEX.md`；DB 不適用性已明確記錄，沒有用空白文件掩蓋缺口。
- skill `quick_validate.py` 已通過；scaffold helper 已完成 Python 編譯檢查，文件相對連結、placeholder 與 staged 敏感資料掃描已通過。

### Git 與同步

- 文件提交：`e6bc960`；Handoff 補充提交：`9202dda`。
- 本機 `HEAD` 與 `origin/main` 已確認一致。
- 最新交接已同步到 Obsidian 與 NotebookLM；後續開發先讀 `docs/PROJECT_DOCUMENTATION_INDEX.md`。

### 未完成與風險

- 完整 GUI 到 `final.mp4` 的乾淨 Windows 視覺 E2E 與完整高品質模型下載仍未驗證。
- 工作樹中原有的影片輸出、翻譯資料、暫存編譯檔與其他未納入里程碑的檔案保持原樣，未刪除、還原或提交。

## 開發文件套件 — 2026-09-18

本專案已建立完整 Markdown 開發設計文件入口：[docs/PROJECT_DOCUMENTATION_INDEX.md](docs/PROJECT_DOCUMENTATION_INDEX.md)。它一次涵蓋需求、SA、SD、UI/UX、RD、資料／DB、API、測試 QA、資安、發布維運、決策追蹤與 Agent 接手；目前版本沒有伺服器 DB，已在 DB 文件明確記錄為不適用，而不是省略。

新開發者先讀文件索引、`SPEC.md`、`DEVELOPMENT_GUIDE.md` 與本檔最新段落，再依改動範圍執行驗證。未來接收一份新的開發規格時，使用全域 skill **完全開發設計指南**（machine id：`complete-development-design-guide`）一次建立或更新整套 Markdown 文件；不要只新增單一需求檔而遺漏設計、測試、資安或發布契約。

本輪文件套件已提交為 `e6bc960 docs: add complete development design set`；推送後應再次確認本機 `HEAD` 與 `origin/main` 相同。

## 收工補充 — 2026-09-18 測試盤點修正

前一版里程碑摘要中的「26 項」只指 APP Python 字幕契約測試。為避免接手者誤把它當成整個專案總數，本輪完整驗證如下：

- `python scripts/install_skill.py`：通過；目前實測 **38 條 CRT skill gotcha**，最低新鮮度門檻為 36。
- `$env:PYTHONPATH=(Resolve-Path 'src').Path; python tests/run_all.py`：**13 項通過**。
- `$env:PYTHONPATH=(Resolve-Path '..\\src').Path; python scripts/test_subtitle_contract.py`：**26 項通過**。
- `cargo test --manifest-path video2crt-app/src-tauri/Cargo.toml --all-targets -- --test-threads=1`：**12 項通過**（11 library unit + 1 integration）。
- 自動化測試合計：**51 項通過**（13 + 26 + 12）。Rust binary harness 顯示 0 tests 是因為它們是可執行驗證工具，仍有編譯驗證，不應重複計入測試案例。
- `cargo test ... model_manager::tests` 的 1 項是窄範圍抽測，不是 Rust 全部測試數。
- gotcha 的唯一完整來源與更新規則見 [`docs/GOTCHA_GUIDE.md`](docs/GOTCHA_GUIDE.md)；各 `output/*/handoff.md` 只屬個案紀錄。

後續文件已同步補上同一份測試矩陣：`video2crt-app/TEST_ACCEPTANCE.md`、`RELEASE.md`、`DEVELOPMENT_GUIDE.md` 與根目錄 `README.md`。本次修正本身只更新文件，不改變程式行為。

## 收工紀錄 — 2026-09-18

### 本輪完成

- 完成 Tauri 桌面版的字幕輸出、翻譯選項、影片標題輸出資料夾與 Credential Manager API Key 流程。
- 修正下載前以 yt-dlp 查標題造成的額外請求，改用 YouTube oEmbed；下載遇到 PO Token、Visitor Data 或 HTTP 429 時，才以 `web_embedded` client 有限重試一次。
- 恢復使用者認可的字幕燒錄設定：NVENC、`p4`、`cq 23`；CRT shader、裁切與渲染策略沒有在這次下載修復中改動。
- 新增並提交 `video2crt-app/DEVELOPMENT_GUIDE.md`，記錄架構、資料流、字幕與影片格式、隱私、驗收、常見問題與維護規則。
- 已提交兩個本輪相關 commit：`b738581` 與 `1abd188`。

### 當輪驗證

- `npm run build`：通過。
- `python -m unittest scripts/test_subtitle_contract.py -q`：26 項通過。
- Rust 下載回退單元測試：通過。
- 同一支 ANA 公開測試影片以 embedded client 實際下載成功；取得 1440×1080 AV1 視訊與 Opus 音訊。
- 手冊已通過 Markdown 結構、敏感字串與 Git diff 空白檢查。

### CRT skill gotcha 版本基準（接手必讀）

- 目前安裝的 `video-crt-geom-libplacebo/SKILL.md` 由 `scripts/install_skill.py` 實測為 **38 條 gotcha**。
- 編號目前為 **0–31、33–38**；**32 號沒有對應條目**。因此不可用「最大編號」推估總數，也不可自行補寫 32 號。
- `EXPECTED_GOTCHA_COUNT = 36` 是自檢的**最低新鮮度門檻**，不是實際總數。看到 `[ALL PASS] Skill v36+` 代表至少達到門檻；接手時仍應記錄實際輸出的 `Found N gotchas in SKILL.md`。
- 本次確認的 `N` 是 **38**。舊段落中出現的 21、23 或 30 都是歷史版本資訊，不得當作目前規格。
- 每次開始新的影片或修正前，從專案根目錄執行 `python scripts/install_skill.py`；若 skill 不存在、依賴缺少或實際條數低於 36，先停止處理並回報。


### Git 狀態

- 分支：`main`；最新 commit：`1abd188 fix: recover YouTube download and document pipeline`。
- 本輪只提交四個檔案：開發手冊、下載協調器、字幕燒錄設定、字幕契約測試。
- 工作樹仍有大量既存的輸出、歷史 handoff、歌詞資料與暫存修改；未刪除、還原、加入 stage 或提交它們。
- `video2crt-app/src-tauri/src/lib.rs` 與 `video2crt-app/src/lib/types.ts` 仍顯示行尾格式的工作樹差異，忽略行尾後無內容差異，未提交。

### 未完成與風險

- 最新程式碼尚未完成一次從 GUI 啟動到 `final.mp4` 的全流程視覺 E2E；不可宣稱整條流程已重新驗收。
- 乾淨 Windows 使用者帳戶的依賴自動準備與 NSIS 安裝流程尚未完成驗證，不可視為可直接散佈的安裝包。
- YouTube 的 PO Token、嵌入權限與連線限制會隨服務端變動；embedded client 只是一條受限回退，不使用帳號 Cookie。

### 接手順序

1. 先讀 `video2crt-app/DEVELOPMENT_GUIDE.md`。
2. 檢查 `git status`，保留所有既有未提交輸出。
3. 若繼續修正，先在新的隔離輸出資料夾完成 GUI 到 `final.mp4` 的 E2E，再動任何 CRT、裁切或字幕設定。
4. 若要處理可散佈性，獨立完成依賴準備與乾淨帳戶安裝驗收；不要與字幕修正混在同一個變更。

---


給未來對話或接手 agent 的完整指引。

## 你是新對話嗎？

**這個專案的**所有** agent 必須自動跑 install_skill.py (gotcha 21)**。這是 **MANDATORY** 不允許跳過的條款 —— 因為新對話環境 state 是 fresh 的，沒驗跑直接開工會遇到 PATH 缺失 / skill 過期 / 工作目錄 reset 等問題。

讀這份檔開始：

1. **第一步（必做）：跑 install_skill.py 驗證環境與 skill**：
   ```bash
   cd "<使用者家目錄>/Documents/Hermes/Video2CRT"
   python scripts/install_skill.py
   ```
   **必須看到** `[ALL PASS] Skill v36+ installed and dependencies OK.`，否則**不要**給用戶任何結論。
   `EXPECTED_GOTCHA_COUNT = 36` 是最低門檻；目前已驗證的實際數量為 38。若 `Found N gotchas` < 36 或 skill / 依賴檢查失敗，先停止並回報。不要把舊文件中的 21、23、30 當作目前總數。
   **這一步不需要 5 秒以外時間，不能跳過**。

2. 看 [`README.md`](README.md) — 專案總覽
3. 看 [`docs/architecture.md`](docs/architecture.md) — 專案結構圖
4. 看 [`docs/index.md`](docs/index.md) — 影片目錄 (output/ + archive/)
5. 看 [`docs/workflow.md`](docs/workflow.md) — 完整工作流程（從 URL 到 final.mp4）
6. 看 [`docs/recipes.md`](docs/recipes.md) — 場景食譜
7. 看 [`docs/troubleshooting.md`](docs/troubleshooting.md) — 常見錯誤排除
8. 看 [`CHANGELOG.md`](CHANGELOG.md) — 歷史紀錄；gotcha 的目前真實數量以 `install_skill.py` 輸出為準。

如果用戶丟新 URL 給你：

1. **立即**跑 `python scripts/install_skill.py`（**即使這對話已經跑過**, 同專案也要再跑一次 — gotcha 21)
2. **工作目錄必須是**：`<使用者家目錄>\Documents\Hermes\Video2CRT\`（**絕對不要**放 Downloads/）
3. 跑 `docs/workflow.md` 的 Stage 1-11
4. 自動套用 skill `video-crt-geom-libplacebo` 的所有 gotcha（目前 **38 條**；編號 0–31、33–38，32 號缺漏）。
5. **顯式宣告** `Found N gotchas in SKILL.md` 證明你已驗證
6. 跑 `python tests/run_all.py`（13 tests，應全綠）

## 工作目錄結構（這是關鍵）

```
<使用者家目錄>\Documents\Hermes\Video2CRT\
├── README.md / LICENSE / VERSION / Makefile    ← 專案 meta
├── docs/                                         ← 所有詳細文件
├── scripts/                                      ← CLI tools
├── src/video2crt/                                ← 可 import 的 Python package
├── tests/                                        ← 13 個 unit tests
├── output/yt_<video-id>/                        ← 完成的影片 + handoff.md
└── archive/yt_<video-id>/                       ← 不完整的影片
```

**所有影片放在** `output/` 或 `archive/` 子目錄，**不再放根目錄的 yt_*/**（v0.5.0 起的標準）。

每個 `output/yt_<video-id>/` 預期結構：
```
source.mp4              # yt-dlp 下載後重新命名
source_16k.wav          # 16kHz mono 音訊給 Whisper 用
final.mp4               # 最終輸出: CRT 效果 + 雙語字幕
raw.mp4                 # libplacebo shader 渲染後
crt.glsl                # 該影片的 GLSL shader
faster_whisper_out*.json # ASR 結果
zh-Hant.srt             # 雙語字幕: line 1 原文 + line 2 繁中
handoff.md              # 該影片處理筆記（如果複雜）
```

**沒有的檔** = pipeline 那步沒跑。

## Skill 載入

skill 本體：`<使用者家目錄>\AppData\Local\hermes\skills\video-crt-geom-libplacebo\SKILL.md`

GitHub mirror: `https://github.com/wuzkkavin/HermesFullSetup/blob/main/skills/video-crt-geom-libplacebo/SKILL.md`

目前基準：本機自檢實測 **38 條 gotcha**，最低門檻為 36；編號 32 缺漏。GitHub mirror 與本段中的舊版本敘述僅供歷史追溯，不能取代本機 `install_skill.py` 的結果。

### 現行 gotcha 快速記憶（38 條）

1-6: 技術基礎（HOOK MAIN, force_original_aspect_ratio, -aspect 16:9, two-step pipeline, GPU, cropdetect）
7-10: Whisper + 字幕基本（faster-whisper, 兩行字幕, 段間距, 跨段驗證）
11: 不準網路找歌詞
12-18: Whisper 幻覺處理（medium, 4-condition filter, no initial_prompt, watermark, chunked, 不要砍 weird, medium.en fallback）
19-20: YouTube CC（允許但要 cross-validate）

詳見 `docs/workflow.md` 與 skill 本身。

## 共同翻譯字典備忘

已處理過的影片中常見 ASR 文字 + 我的翻譯：

### 日文 live 演唱會常用
- `胸が熱くなる` → 胸口發燙
- `思い出すと今も Dreaming Rainbow` → 回想起來至今仍是夢中彩虹
- `遠い日の中で` → 在遙遠的日子裡
- `南風が消した` → 南風吹散的
- `白いベストの向こうの道は` → 白色背心對面道路
- `遠い虹を割られたの` → 遙遠的彩虹被割裂了

### 英文 live MC（看起來 weird 但是真的，gotcha 17）
- `Bullshit` → 胡說
- `Kick the hop the shit you` → 踢吧你這混蛋
- `Get the heart to shoot you` → 用槍瞄準你
- `The danger you can't beat me` → 危險打不倒我

### YouTube end-screen 浮水印（要過濾掉，gotcha 15）
- `サブタイトル チャンネル登録してね!` → YT 訂閱請求
- `字幕 請訂閱本頻道` → YT 訂閱請求
- 含 "チャンネル登録" / "subscribe" / "登録して" / "高評価" / "いいね" 關鍵字 — 過濾掉

### Whisper prompt leakage（要過濾掉，gotcha 14）
- `日本語 ロックライブ` → prompt 殘留
- `歌詞・歌詞・歌詞...` → repetitive spam

## src/video2crt/ Python package

可 import 的模組（給未來重用）：

```python
from video2crt.subtitle import is_skip, is_yt_watermark, fmt_time, build_srt
from video2crt.asr import extract_audio, transcribe
from video2crt.pipeline import render_raw, burn_subtitles, mux_audio
```

每個函式對應 `docs/workflow.md` 的一個 stage。**不要 reimplement** — 直接用。

## 已知限制

- Whisper 對**日文 live 演唱會** 偶有幻覺 → 4-condition filter
- Whisper **medium-multilingual 對後段**（>50% 影片長度）降準 → chunked + medium.en fallback
- `libass` fade-out 0.3s 殘影 → 0.5s gap 解決
- 燒字幕**不能跟 libplacebo 同一步** → 兩步驟管線

## 未完成的待辦

1. `output/yt_southern_cross/` — 沒有 final.mp4，需要 finish
2. `archive/yt_videoB/` — 缺 source.mp4，需要重新下載
3. `archive/yt_CaCSuzR4DwM/` — 缺 final.mp4，需要 finish
4. 9 支影片缺 handoff.md（見 `docs/index.md`）
5. 比較輸出檔案品質，淘汰粗顆粒版本

## ⏸ GUI 開發計畫（規劃中，等 user 確認才實作）

**2026-09-04 user 問**：能不能把 video2crt 做成 Windows APP，貼網址就轉 CRT 影片？

**已寫**：完整開發手冊 [`docs/DEV_GUIDE.md`](docs/DEV_GUIDE.md)（約 430 行，方案 B = Tkinter GUI）。

**新對話 AI agent 看到這段時**：
1. **不要直接寫 GUI code**，先讀 `docs/DEV_GUIDE.md` 確認 plan
2. 問 user：「要繼續 GUI 開發嗎？」
3. **只有 user 明示確認**才開始 Phase 1（orchestrator.py）
4. **TDD 順序**：先寫測試 → 再寫 code
5. 別又像之前沒驗證先做（user 反覆強調「先確認再執行」）

**GUI 設計重點**：
- 單一主視窗 600x500
- Tkinter + ttk standard widgets（跨 Windows 10/11 一致）
- WorkerThread 在 background 跑 9 stages
- queue.Queue 從 worker 傳訊息到 UI（Tkinter 不是 thread-safe）
- Cancel button + subprocess.terminate
- Settings 存 %LOCALAPPDATA%/Video2CRT/settings.json
- 用 pyinstaller --onefile 打包成 .exe

**預計時程**：約 1-2 週（10 phases，TDD）。

**開發順序**（不能跳）：
Phase 1 orchestrator → 2 translation → 3 GUI 骨架 → 4 worker queue → 5 cancel → 6 settings → 7 file dialog → 8 e2e → 9 build exe → 10 manual test

## 常見 user 反饋模式

| 偏好 | 解釋 |
|---|---|
| Traditional Chinese (Taiwan) | 全部回應繁中（locale zh-TW） |
| 不要重複犯同樣錯 | 每次學新 gotcha → update skill → push git |
| 完全驗證結果 | 「不要自己瞎掰說完成了」 |
| 不要瞎填歌詞 | ASR-only，YouTube CC 允許但要 cross-validate |
| 字幕兩行 | 原文 + 繁中，**不加第三行括號註釋** |
| 0.5s end-margin + 0.5s gap | 防止 libass fade-out ghost |

## 你接手時，**絕對不要**：

0. ❌ **跳過 install_skill.py**（gotcha 21 MANDATORY）。即使對話前面已跑過、同個專案、看似環境沒變，也要再跑一次。**永遠不跳過**。
0a. ❌ **不宣告 `Found N gotchas in SKILL.md`** 證明已驗證。每個新對話都應該在第一次回應就顯示這行。

1. ❌ 砍 weird 文字像 Bullshit、Kick the hop (gotcha 17)
2. ❌ 用 avg_logprob 當獨立過濾 (gotcha 13)
3. ❌ 設 `initial_prompt=` 給 Whisper (gotcha 14)
4. ❌ 從 j-lyric.net / animesonglyrics.com / genius.com 抓歌詞 (gotcha 11)
5. ❌ 在 libplacebo shader 用 `//!HOOK RGB` (gotcha 1)
6. ❌ 把字幕跟 libplacebo 同一個 ffmpeg pass 跑 (gotcha 4)
7. ❌ mux 後忘記加 `-aspect 16:9` (gotcha 3)
8. ❌ 只看前 30 秒的字幕就說完成 (gotcha 10)
9. ❌ 把影片放 `Downloads/`（必須是 `output/yt_*/`，gotcha 22）
10. ❌ 沒跑 `install_skill.py` 就開工（gotcha 21，已重複列出，強調）

## 你接手時，**必須**：

0. ✅ **先跑 `python scripts/install_skill.py`** 並看到 `[ALL PASS]`（每個新對話都跑 — gotcha 21）
1. ✅ 工作目錄在 `Documents/Hermes/Video2CRT/output/yt_<id>/` (gotcha 22)
2. ✅ 用 faster-whisper **medium** 作為 primary ASR
3. ✅ 對 3+ 分鐘影片跑 chunked ASR (gotcha 16)
4. ✅ 對 multilingual 沉默 30+ 秒段跑 medium.en fallback (gotcha 18)
5. ✅ 視覺驗證**完整跨段**抽樣 (gotcha 10)
6. ✅ 字幕從 0:00 開始（如果 Whisper 從 0:00 給的話）
7. ✅ handoff.md 寫處理細節
8. ✅ `docs/index.md` / `CHANGELOG.md` / `README.md` 隨之更新
9. ✅ 任何新 gotcha 立即 commit + push 到 GitHub
10. ✅ 跑 `python tests/run_all.py`，13 tests 應全綠

## 重要：井水不犯河水

**Video2CRT 專案只 touch 以下路徑**：
- `<使用者家目錄>\Documents\Hermes\Video2CRT\`（工作目錄）
- `<使用者家目錄>\AppData\Local\hermes\skills\video-crt-geom-libplacebo\`（skill 定義）
- `<使用者家目錄>\AppData\Local\hermes\Video2CRT\`（HermesFullSetup mirror 子目錄）

**不要碰**：
- `<使用者家目錄>\opencode\workspace\opencode-full-setup\`（OpenCode CLI 工作目錄 —— 另一條 worktree）
- 任何其他 local git repo

## ⚠ 顯示檔案給用戶：永遠用 MEDIA: token

**User correction 2026-09-06**：以前對話可以「直接顯示影片在介面上」，這次 agent（我）剛開始說「沒有 GUI 介面、只能給路徑」，**這是錯的**！

**正確做法**：
- **永遠用 `MEDIA: <absolute path>` 寫在對話訊息中**（不是 echo 到 terminal）
- Hermes Desktop 識別 `MEDIA:` 後自動渲染檔案預覽
- **絕對不要說「我沒 GUI」或「只能給路徑」**——先試 MEDIA: token
- 如果 MEDIA: 不渲染，**fallback**：`desktop_preview` 工具 `action=open` + `url=path`

**範例**：
```
MEDIA: <使用者家目錄>\Documents\Hermes\Video2CRT\output\yt_XXX\final.mp4
```

**為什麼 agent 容易忘**：MEDIA: 看起來像純文字字串，agent 直覺會 echo 到 terminal。但 Hermes Desktop 是**解析 agent 對話訊息**（不是 terminal 輸出）。

如果 Hermes 桌面 sidebar 同時列出 `Video2CRT` 和 `opencode-full-setup`，**那是兩個獨立專案**。Video2CRT agent 只處理 Video2CRT 範圍，碰到 opencode 路徑要**立刻停**並回報用戶。User clarification 2026-09-04：「**你（Video2CRT）是他（OpenCode）的，井水不犯河水**」。

---

最後更新：2026-09-06（v0.5.0 重新組織 + MEDIA: token 規則）
歷史交接紀錄：當時的 30 個 gotcha 已固化；目前總數已增至 38，請以本檔開頭的版本基準與本機自檢為準。專案結構：方案 C + MIT License。

---

## 2026-09-17 Video2CRT 字幕與翻譯收工紀錄

### 已完成

- 字幕輸出可選：只有原文、原文加繁體中文、無字幕；中文原片只保留繁中單行。
- 翻譯可選：本機翻譯、本機失敗時雲端備援、純雲端翻譯。純雲端或雲端備援未設定憑證時，介面會直接帶往設定畫面。
- 雲端憑證以 Windows Credential Manager 保存；輸入欄位採密碼遮罩，未設定憑證時本機功能仍可使用。
- 修正雲端回傳思考內容或非繁中文字時被帶入字幕的情形；不合格翻譯會被拒絕。
- 修正 faster-whisper 在低解析音樂影片的單字時間戳空陣列錯誤：僅針對該錯誤，自動關閉單字時間戳並以本機 ASR 重試。
- 字幕燒錄改用與 CRT 主轉檔相同的 libx264 CRF 18，避免低解析來源因第二次編碼而明顯變糊。
- 預設輸出為桌面下以影片標題命名的新資料夾；既有資料不覆寫、不使用 YouTube ID 當資料夾名稱。

### 本次異動

- Tauri 編排、Windows 憑證保存與 MiniMax 翻譯防護。
- Python 字幕處理、YouTube 原語字幕選擇、本機翻譯與 ASR 備援。
- 選項頁、進度頁與型別定義。
- 字幕契約回歸測試、既有環境檢查的 yt-dlp 路徑辨識。
- 內嵌 OpenCC 繁簡轉換資料；不含使用者資料或憑證。

### 驗證

- `python -m unittest scripts/test_subtitle_contract.py -q`：26 項通過。
- `npm run build`：通過。
- `cargo test --lib`：9 項通過。
- `python tests/run_all.py`：13 項通過（已更新 yt-dlp 偵測路徑）。

### Git 狀態與提交範圍

- 僅提交應用程式、測試、必要離線 OpenCC 資料與本交接文件。
- 不提交既有的影片輸出、刪除紀錄、原始影音、暫存檔、日誌或任何憑證。

### 已知限制與下一步

- 低品質原始影片的細節上限仍受原始來源限制；本次修正的是字幕燒錄不再額外降低畫質。
- 建議下一次以一支低解析影片分別測試原文、雙語本機、雙語純雲端三種模式，確認字幕內容與畫質的實機觀感。
# Video2CRT — 2026-09-18 Windows App 發布里程碑

## 本輪目的

完成可散佈 Windows APP 的收工交接：把必要 runtime 與標準模型放入 NSIS 安裝包，加入高品質模型的使用者確認式安裝，補齊下一個 agent 可直接使用的規格、操作、維護、測試與發布文件，並完成當輪授權的 commit/push。

## Current truth

- APP 版本：`0.1.0`，Windows x64，Tauri 2 + React/Vite + Rust/Tokio + Python sidecar。
- 正式安裝包：`video2crt-app/dist-distributable/Video2CRT_0.1.0_x64-setup.exe`。
- 安裝包已包含 Python 3.12 embeddable、ffmpeg、Node.js、官方 yt-dlp、離線 WebView2、OpenCC 與標準 ASR／翻譯模型。
- 高品質模型不是靜默下載：NSIS 安裝程序顯示下載／略過選項；確認後由 bundled Python 下載到使用者資料夾，完成 manifest 後才切換。主程式運行介面不顯示模型選擇。
- 發布包 SHA-256（2026-09-18）：`96AAEF70137210371A2E471727F7E4FDE9CECE3DA02B0047A5228265B8B66232`。

## 本輪變更與權威文件

- 模型管理：`video2crt-app/src-tauri/installer-hooks.nsh`、`video2crt-app/scripts/install-large-model.py`、`video2crt-app/src-tauri/src/model_manager.rs`；`ModelInstallDialog.tsx` 已移除。
- Tauri command、runtime 資源與模型路徑：`video2crt-app/src-tauri/src/lib.rs`、`orchestrator.rs`。
- 發布腳本與 runtime staging：`video2crt-app/scripts/build-distributable.ps1`、`stage-models.py`。
- 文件入口：`video2crt-app/README.md`、`SPEC.md`、`USER_GUIDE.md`、`DEVELOPMENT_GUIDE.md`、`TEST_ACCEPTANCE.md`、`RELEASE.md`、`AGENT_GUIDE.md`。

## 當輪驗證

- `npm run build`：通過。
- `cargo check --manifest-path src-tauri/Cargo.toml`：通過（既有 warnings）。
- `python scripts/install_skill.py`：通過，實測 38 條 gotcha，`[ALL PASS]`。
- `python tests/run_all.py`：13 項通過。
- `python scripts/test_subtitle_contract.py`（搭配 repo `src`）：26 項通過。
- `cargo test --all-targets -- --test-threads=1`：12 項通過（11 library unit + 1 integration）。
- 自動化測試合計：51 項通過；`model_manager::tests` 的 1 項只是窄範圍抽測。
- `powershell -File scripts/build-distributable.ps1`：通過，NSIS 產物存在。
- silent install：exit 0；安裝後 `video2crt.exe` 啟動並持續執行超過 5 秒。
- 尚未驗證：乾淨 Windows 使用者帳戶、完整 GUI 到 `final.mp4` 視覺 E2E、實際下載完整約 3.2 GB 高品質模型。

## Git 與外部狀態

- 工作分支：`main`。
- 本輪只應提交 APP 原始碼、發布腳本、文件與 lockfile；父層影片輸出、原始影音、日誌、模型快取與暫存檔不得提交。
- `origin` 目前由 GitHub 回報為公開 repository；本輪不可新增私密路徑、憑證或使用者資料。若日後要求私有化，需另行明確處理 repository visibility。
- commit/push 由老吳在本輪訊息中明確授權；完成後要再次比較本機 `HEAD` 與 `origin/main`。
- 本輪發布 commit：`54e8055`，文件補充 commit：`270cd46`；兩者均已推送至 `origin/main`，目前本機與遠端 SHA 為 `270cd46e849a9efb285b4acc142be0d8f8efd67e`。

## 下一個 agent 的安全起手式

1. 閱讀 `video2crt-app/AGENT_GUIDE.md`、`SPEC.md`、`RELEASE.md`、`TEST_ACCEPTANCE.md`。
2. 執行 `git status --short`，確認不要碰父層既有輸出。
3. 若要改模型下載，先讀 `model_manager.rs` 與其單元測試；不要直接啟動 3.2 GB 下載。
4. 若要宣稱完整發布，先補乾淨帳戶安裝、模型下載與 GUI→`final.mp4` E2E 證據。

---
