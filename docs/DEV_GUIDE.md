# Video2CRT Developer Guide

> **完整開發手冊** — 給未來要維護或擴展 Video2CRT 的 developer 或 AI agent 看。
> **最後更新**：2026-09-04（規劃中，GUI 未實作）
> **目標讀者**：能跑 Python + ffmpeg + Git 的人員；接班 AI agent

---

## 0. 30 秒總覽

Video2CRT 是一個把 YouTube 影片 / 本地影片**轉成 1920×1080 CRT-style MP4**（RGB phosphor dotmask + 掃描線 + 雙語字幕）的工具集。目前以 **CLI + agent 對話模式**運作（`scripts/run_whisper.py` + `src/video2crt/` pipeline + skill guidance）。

目標是**做一個 Windows 桌面 GUI**，讓 user 貼 URL 或拖檔 → 按按鈕 → 等 → 看 final.mp4。**這份文件是寫程式之前的設計書**，等你確認後才開始實作。

---

## 1. 現有程式碼盤點（已經完成的部分）

### 1.1 Pipeline 流程（end-to-end）

```
YouTube URL or local video file
   ↓  Stage 1: yt-dlp 下載（或讀 local file）
   ↓  Stage 2: ffprobe + cropdetect 偵測黑邊
   ↓  Stage 3: ffmpeg → source_16k.wav（給 Whisper 用）
   ↓  Stage 4: faster-whisper medium ASR（多語言，可加 medium.en fallback）
   ↓  Stage 5: 寫 zh-Hant.srt（兩行：原文 + 繁中）
   ↓  Stage 6: ffmpeg libplacebo shader → raw.mp4（CRT 效果）
   ↓  Stage 7: ffmpeg burn subtitles → subtitled.mp4
   ↓  Stage 8: ffmpeg -aspect 16:9 mux audio → final.mp4
   ↓  Stage 9: 視覺驗證多時間點
```

### 1.2 已寫好的檔案

| 檔案 | 行數 | 角色 |
|---|---|---|
| `scripts/install_skill.py` | 198 | 環境 + skill + gotcha 數量驗證 |
| `scripts/run_whisper.py` | 219 | 3-stage Whisper runner CLI |
| `scripts/crt.glsl` | 3 | RGB dotmask + scanline shader（一行 GLSL） |
| `src/video2crt/__init__.py` | 5 | Package init (version) |
| `src/video2crt/asr.py` | 56 | Whisper ASR extraction + transcription |
| `src/video2crt/subtitle.py` | 86 | SRT builder（4-condition filter + 段間距控制） |
| `src/video2crt/pipeline.py` | 70 | ffmpeg 渲染、燒字幕、mux |
| `tests/test_install_skill.py` | 57 | skill 驗證測試 |
| `tests/test_whisper_stages.py` | 111 | 4-condition filter / watermark / prompt leakage 測試 |
| `tests/run_all.py` | 25 | unittest discovery runner |
| `tests/__init__.py` | 2 | tests 套件標記 |

**合計 832 行**（含 crt.glsl + 測試 195 行）

### 1.3 已建立但**未串接**的部分

```
❌ Stage 1 (yt-dlp download) — 在 CLI scripts/run_whisper.py 沒完整實作
❌ Stage 2 (cropdetect integration) — 知道怎麼跑但 pipeline.py 沒自動 crop
❌ Stage 5 SRT build (Chinese translations) — subtitle.py 有 build_srt() 但無 translation dict
❌ Stage 6 → 9 串接 — pipeline.py 有 render_raw, burn_subtitles, mux_audio 三個獨立函數但沒有合併呼叫的 orchestration 腳本
❌ convert.bat 或 main.py 入口 — 完全沒寫
```

**現狀**：每個 stage 是獨立函數，**沒有「一鍵完整跑」**。Agent 對話手動組合。**GUI 必須先寫 orchestration**。

---

## 2. GUI 開發方案比較

| 維度 | A. convert.bat (CLI batch) | B. Tkinter (Python GUI) | C. PyQt6 (Qt) | D. Electron |
|---|---|---|---|---|
| 開發時間 | 30 min | 1-2 天 | 3-5 天 | 1-2 週 |
| 套件 | 無（純 batch） | **內建**（stdlib） | pip install | npm + Node.js |
| 視窗品質 | 命令列黑窗 | OK（醜但能用） | 美觀（native widgets） | 最美（HTML/CSS） |
| 跨平台 | 僅 Windows | Win/Mac/Linux | Win/Mac/Linux | Win/Mac/Linux |
| 套件大小 | 0 KB | 0 KB | ~50 MB | ~80 MB Electron runtime |
| 打包成 exe | 不需要 | pyinstaller 30 min | pyinstaller 1 hr | electron-builder 半天 |
| 學習曲線 | 0 | 低 | 中 | 高 |
| 維護成本 | 0 | 低 | 中 | 高 |
| 適用場景 | 進階用戶 | 一般 user 友善 | 進階 user | 商業產品 |

**user 選了 B (Tkinter)**。

### 2.1 Tkinter 為何合理

- Python stdlib，**0 額外安裝**
- 已驗證環境有 Python 3.11 + faster-whisper（install_skill.py 全綠）
- 跨平台但只 deploy Windows
- 用 pyinstaller 一行 `pyinstaller --onefile gui.py` 變 exe
- 對個人工具 OK，**不追求 UI 美觀**（已確定）

---

## 3. Tkinter GUI 詳細架構設計

### 3.1 視窗佈局（單一主視窗 ~600x500）

```
+---------------------------------------------+
| Video2CRT v0.5.0                    [_][□][X] |
|---------------------------------------------|
| [URL or Drop file here]:                    |
| [_________________________________] [Browse] |
|                                             |
| Source:  (o) YouTube URL  ( ) Local file     |
|         ( ) YouTube CC (cross-validate)      |
|                                             |
| Language:  [auto-detect ▼]                  |
|            options: auto / ja / en / zh / ko |
|                                             |
| Output dir: [output/yt_XXX_____________]    |
|             (auto-generate from URL or filename) |
|                                             |
| [ Download ] [ Convert ] [ Open Output ]     |
|                                             |
| Progress: [████████░░░░░░░░░] 50%            |
| Stage: Running Whisper medium ASR...        |
| Log:                                          |
| ┌─────────────────────────────────────────┐ |
| │ [15:30:42] yt-dlp: downloaded 80MB         │ |
| │ [15:31:10] cropdetect: 1920:820:0:130     │ |
| │ [15:31:45] whisper: 55 segments (prob 0.85+)│ |
| │ [15:32:30] SRT built: 55 entries           │ |
| │ [15:33:00] libplacebo render: 32% stretch   │ |
| │ [15:34:30] burn subtitles: OK              │ |
| │ [15:35:00] mux audio -aspect 16:9: OK      │ |
| │ [15:35:30] final.mp4 ready: 282 MB          │ |
| └─────────────────────────────────────────┘ |
|                                             |
| Status: Ready.                             |
+---------------------------------------------+
```

### 3.2 核心 class 設計

```python
# src/video2crt/gui.py

class Video2CRTApp:
    """Main Tkinter window controller."""

    def __init__(self):
        self.root = Tk()
        self._build_ui()
        self._wire_callbacks()
        # ... state
        self.worker: Optional[WorkerThread] = None

    def _build_ui(self):
        """Build all widgets from ttk widgets."""
        # - url_entry
        # - source_type_var (radiobutton)
        # - language_var (combobox)
        # - progress_bar
        # - log_text (ScrolledText)
        # - buttons: download_btn, convert_btn, open_btn

    def _wire_callbacks(self):
        self.download_btn.config(command=self._on_download)
        self.convert_btn.config(command=self._on_convert)
        self.open_btn.config(command=self._on_open_output)

    def _on_download(self):
        # validate URL or file path
        # generate output dir from id
        # launch WorkerThread in "download-only" mode
        pass

    def _on_convert(self):
        # already downloaded
        # launch WorkerThread in "full-convert" mode
        pass

    def _on_open_output(self):
        # open Windows Explorer at output dir
        # or play final.mp4 in default player
        pass


class WorkerThread(threading.Thread):
    """Background worker. NEVER touch tk from worker - use queue.Queue."""

    def __init__(self, app, job: Job):
        super().__init__(daemon=True)
        self.app = app
        self.job = job

    def run(self):
        try:
            self._stage1_download()
            self._stage2_cropdetect()
            self._stage3_extract_audio()
            self._stage4_whisper()
            self._stage5_build_srt()
            self._stage6_render_raw()
            self._stage7_burn_subtitles()
            self._stage8_mux_audio()
            self._stage9_verify()
        except Exception as e:
            self._log(f"[FATAL] {e}")
        finally:
            self._done()
```

### 3.3 關鍵設計約束

| 約束 | 原因 |
|---|---|
| **Worker thread 用 queue.Queue 傳訊息回 UI** | Tkinter 不是 thread-safe，**不能在 worker thread 直接呼叫 widget 方法** |
| **Stage 1-9 全部跑在 worker thread** | 不能 block UI（會當機） |
| **Cancel button** | 1-3 min 處理時間要有取消選項 |
| **Progress 是預估值，不是精確** | 每個 stage 時間差異大 |
| **Log 是 stdout 重定向，不是 buffered** | 即時顯示給 user |
| **錯誤必須 recoverable**：Stage 失敗可從中斷處重試，不重新從頭 | Video2CRT pipeline 可能 5 min 起跳，重試很重要 |
| **路徑用 forward slash 或 pathlib.Path** | 避免 Windows backslash 在 shell 跳脫 |
| **CJK 字型** | Use `Microsoft JhengHei` system font, fallback to `TkDefaultFont` |

### 3.4 進度回報（from worker to UI）

```python
# In WorkerThread
def _log(self, msg: str):
    self.app.message_queue.put(("log", msg, time.time()))

def _progress(self, stage_name: str, percent: float):
    self.app.message_queue.put(("progress", stage_name, percent))

def _done(self):
    self.app.message_queue.put(("done", self.job.output_dir, time.time()))
```

```python
# In Video2CRTApp main loop (called via root.after(100, ...))
def poll_messages(self):
    while not self.message_queue.empty():
        kind, payload, ts = self.message_queue.get_nowait()
        if kind == "log":
            self.log_text.insert("end", f"[{time.strftime('%H:%M:%S')}] {payload}\n")
            self.log_text.see("end")
        elif kind == "progress":
            self.progress_bar["value"] = payload * 100
            self.status_label.config(text=f"Stage: {payload}")
        elif kind == "done":
            self.on_job_done(payload)
    self.root.after(100, self.poll_messages)
```

### 3.5 取消按鈕（kill switch）

```python
def _on_cancel(self):
    if self.worker and self.worker.is_alive():
        # cooperative cancellation: set a flag
        self.worker.cancel_requested = True
        # OR for stubborn blocking calls (subprocess): os.kill
        # We need to track subprocess.Popen and terminate it
        if self.worker.current_popen:
            self.worker.current_popen.terminate()
            self.worker.current_popen.wait()
        self._log("[INFO] Job cancelled by user")
```

### 3.6 設定存檔（跨 session 記住選項）

```python
import json
from pathlib import Path

class Settings:
    def __init__(self, path: Path = Path.home() / ".video2crt_settings.json"):
        self.path = path
        self.data = self._load()

    def _load(self):
        if self.path.exists():
            return json.loads(self.path.read_text(encoding="utf-8"))
        return {"language": "auto", "source_type": "youtube"}

    def save(self):
        self.path.write_text(json.dumps(self.data, indent=2), encoding="utf-8")

    def get(self, key, default=None):
        return self.data.get(key, default)
```

---

## 4. 新檔案結構（規劃）

```
C:\Users\<you>\Documents\Hermes\Video2CRT\
├── 現有檔案 (略)
│
├── src/video2crt/
│   ├── (existing) __init__.py, asr.py, subtitle.py, pipeline.py
│   ├── (NEW) gui.py              ← Tkinter 主視窗 + WorkerThread (約 400-600 行)
│   ├── (NEW) orchestrator.py     ← 完整 9-stage pipeline 串接 (約 200-300 行)
│   └── (NEW) translation.py      ← ASR text → Trad Chinese dictionary (目前沒寫)
│
├── scripts/
│   ├── (existing) install_skill.py, run_whisper.py, crt.glsl
│   ├── (NEW) run_gui.bat         ← 雙擊啟動 GUI
│   └── (NEW) build_exe.bat      ← pyinstaller 打包 .exe
│
├── tests/
│   ├── (existing) ...
│   ├── (NEW) test_orchestrator.py     ← Stage 1-9 串接測試 (使用 mock video)
│   ├── (NEW) test_gui_smoke.py         ← Tkinter 視窗能初始化不 crash
│   ├── (NEW) test_translation.py      ← 翻譯字典的 lookup 邏輯
│   ├── (NEW) test_worker_queue.py      ← Worker → queue → UI 流程
│   └── (NEW) test_cancel_button.py    ← 取消能殺掉 subprocess
│
├── docs/
│   ├── (existing) ...
│   └── (NEW) GUI_USAGE.md             ← 給 user 看的 GUI 使用手冊
│
├── dev_artifacts/                     ← 開發暫存（不入 git）
│   ├── dist/
│   │   └── Video2CRT.exe              ← pyinstaller 產物
│   └── build/
│
└── gui.spec                           ← pyinstaller 設定檔
```

**估計新增 ~800-1000 行**（含測試）

---

## 5. 開發步驟 Checklist

> 每步都先寫測試，再寫 code（TDD）。

### Phase 0 — 規劃（本份文件）
- [x] 寫 DEV_GUIDE.md（本檔）
- [x] 等 user 確認
- [ ] 更新 HANDOFF.md 加入「未來 GUI 開發計畫」段落

### Phase 1 — Orchestrator（GUI 不依賴）
- [ ] 寫 `src/video2crt/orchestrator.py`，把 Stage 1-9 串成單一 `run_pipeline()` 函數
- [ ] 寫 `tests/test_orchestrator.py`：用 mock video 跑完 9 stages 確認順序 + 產物
- [ ] 確認 Stage 1-9 全 runnable 從 CLI（python -m video2crt.orchestrator）

### Phase 2 — Translation
- [ ] 寫 `src/video2crt/translation.py`，含 Fuji Kaze "Kirari" / Sara / etc 翻譯字典
- [ ] 寫 `tests/test_translation.py`：lookup + normalize

### Phase 3 — Tkinter GUI 骨架
- [ ] 寫 `src/video2crt/gui.py`：Video2CRTApp class + WorkerThread
- [ ] 寫 `tests/test_gui_smoke.py`：import 模組 + 建立 Tk root + 銷毀
- [ ] **測試要在 CI 能跑**（headless `xvfb-run` 或 skipif-no-display）

### Phase 4 — Worker Thread + 訊息佇列
- [ ] 完善 WorkerThread，9 stages 全跑在 worker
- [ ] 用 queue.Queue 傳訊息
- [ ] 寫 `tests/test_worker_queue.py`：mock queue 驗證訊息流

### Phase 5 — Cancel 機制
- [ ] 取消按鈕能 terminate subprocess + 殺 worker
- [ ] 寫 `tests/test_cancel_button.py`：假裝跑 slow stage，按 cancel

### Phase 6 — Settings 存檔
- [ ] Settings class 寫讀 .json
- [ ] 寫 `tests/test_settings.py`

### Phase 7 — File Dialog + Drag-and-Drop
- [ ] Browse 按鈕用 `tkinter.filedialog`
- [ ] Drag-and-drop 用 `tkinterdnd2` (需要 `pip install tkinterdnd2`)

### Phase 8 — 整合測試（end-to-end）
- [ ] 用 mock 30s 短影片跑完整 GUI 流程
- [ ] 確認 final.mp4 真的有 CRT 效果 + 字幕

### Phase 9 — 打包
- [ ] 寫 `gui.spec`
- [ ] 寫 `scripts/build_exe.bat`：呼叫 pyinstaller
- [ ] 產出 `dist/Video2CRT.exe`

### Phase 10 — Manual Test Plan
- [ ] 把 Video2CRT.exe 給 user 在真實 Windows 跑
- [ ] 測試：貼 URL → 跑完 → 開 final.mp4
- [ ] 測試：拖檔 → 跑完 → 開 final.mp4
- [ ] 測試：按取消 → subprocess 殺掉
- [ ] 測試：CCC 八支影片都能正常轉

---

## 6. 完整測試計畫

### 6.1 單元測試（unittest）

| 測試 | 數量 | 範圍 |
|---|---|---|
| `test_install_skill.py` | 6 | skill presence, gotcha count, ffmpeg binaries, libplacebo, vulkan, nvenc |
| `test_whisper_stages.py` | 7 | 4-condition filter, prompt leakage, watermark detection, real segments not filtered |
| `test_translation.py` (NEW) | ~10 | dictionary lookup, normalize whitespace, missing key handling |
| `test_gui_smoke.py` (NEW) | ~5 | Tk root initialize, widget creation, class import without crash |
| `test_worker_queue.py` (NEW) | ~8 | queue message ordering, types, threading.Thread lifecycle |
| `test_settings.py` (NEW) | ~4 | load/save roundtrip, missing file, corrupt file |
| `test_cancel_button.py` (NEW) | ~3 | mock slow subprocess, terminate, ensure killed within 2s |
| **合計** | **~43** | (現有 13 + 新增 ~30) |

### 6.2 整合測試

| 測試 | 內容 | 跑的環境 |
|---|---|---|
| `test_orchestrator_e2e.py` | 完整 Stage 1-9 用 mock 30s video | local CI |
| `test_gui_full_flow.py` | Tkinter 視窗啟動 → mock download → 進度條到 100% → final.mp4 存在 | local, headless |

### 6.3 手動測試清單

```
□ 啟動 Video2CRT.exe
□ 視窗出現，沒 crash
□ 貼 YouTube URL → 按 Download → 開始下載，progress 跳動
□ 下載完 → 按 Convert → 跑 9 stages → 看 log 跳出訊息
□ 跑完 → final.mp4 檔案存在於 output/yt_XXX/
□ 按 Open Output → Windows Explorer 打開 output/yt_XXX/
□ 雙擊 final.mp4 → Windows Media Player 開啟，看到 CRT 效果 + 字幕
□ 拉本地檔案 → 拖入視窗 → 按 Convert → 處理
□ 處理到一半按 Cancel → subprocess 殺掉，視窗可再用
□ 重啟 app → 上次的設定（語言、輸出目錄）保留
```

### 6.4 Regression Tests

每次 commit 都跑：
```bash
cd "C:/Users/<you>/Documents/Hermes/Video2CRT"
python tests/run_all.py        # ~43 unit tests
python tests/test_orchestrator.py  # 1-2 個 e2e
```

CI 失敗**不要 merge**。

### 6.5 Test Data

需要**mock 影片**給 e2e 用：
- `tests/fixtures/sample_30s_4x3.mp4` （20 MB）
- `tests/fixtures/sample_30s_letterbox.mp4` （故意 1920×820 內容測 gotcha 24）
- `tests/fixtures/sample_with_subtitles.mp4` （已燒字幕的，測試不要覆蓋）

**這些 mock 影片不入 git**（.gitignore 加 `tests/fixtures/`），改用 generation script：

```python
# tests/fixtures/generate.py
import subprocess, os
os.makedirs("tests/fixtures", exist_ok=True)
# 從 source 抽 30s 片段 + crop 模擬 4:3 + 上下黑邊
subprocess.run(["ffmpeg", "-y", "-i", "source.mp4", "-ss", "30", "-t", "30",
                "-vf", "crop=1440:1080:240:0,scale=1920:1080:flags=neighbor,setsar=1",
                "tests/fixtures/sample_30s_4x3.mp4"])
```

---

## 7. 風險與決策點

### 7.1 已知風險

| 風險 | 嚴重性 | 緩解 |
|---|---|---|
| Tkinter 介面在 Windows 10 vs 11 不同版本會 pixel-perfect 不同 | 中 | 用 ttk 標準 widget，不自繪 |
| faster-whisper model 第一次下載 ~1.5 GB | 中 | GUI 顯示「Downloading model...」進度條 |
| Worker thread 沒人 kill 會 leak | 中 | daemon=True，process 結束時自動回收 |
| ffmpeg 跑 5 分鐘 user 等不了 | 中 | Cancel button + 縮短預估時間 |
| pyinstaller exe 被 Windows Defender 誤判 | 中 | code-sign certificate（要買） |
| Stage 7 burn subtitle 在 5+ hour video 會 OOM | 低 | 不支援 5+ hour，warn |
| Crop + stretch 對人物變形 user 不滿意 | 中 | 給 A/B/C 預覽（gotcha 24 經驗） |
| 字幕兩行對窄字幕 (短行) 可能太擠 | 低 | 用 FontSize 24 + outline 2px |

### 7.2 待決策（user 確認前不寫 code）

1. **GUI 用什麼 icon？**
   - 預設：Python 預設圖示
   - 建議：找一張 CRT 風格 icon（30 分鐘可做）

2. **支援哪些檔案格式？**
   - 預設：mp4（yt-dlp 下載的）
   - 可加：.mkv / .webm / .avi

3. **是否要任務佇列（batch）？**
   - v1：1 個 job 一次
   - v2：可加入多個 URL 排隊處理

4. **是否打包成單一 exe？**
   - 簡單：發布 Python + bat
   - 完整：pyinstaller --onefile（推薦，但 exe ~30 MB）

5. **Settings 存哪？**
   - `~/.video2crt_settings.json` (user home，跨 project)
   - `%LOCALAPPDATA%/Video2CRT/settings.json` (Windows convention)

---

## 8. 何時結束這份文件

- [x] 寫完整盤點（Section 1-3）
- [x] 寫測試計畫（Section 4-6）
- [x] 寫開發步驟（Section 5）
- [x] 列風險（Section 7）
- [ ] **等 user review + 確認**（Section 7.2 決策點）
- [ ] 然後才開始 Phase 1

---

## 9. 給未來新對話 AI agent

**看到這份 DEV_GUIDE.md 時**：
1. 讀 HANDOFF.md 確認 gotcha 沒違反
2. 讀這份 DEV_GUIDE.md 確認 plan
3. **不要直接寫 code**，先問 user：「要繼續 GUI 開發嗎？」
4. 用戶確認後，從 Phase 1 開始（TDD：先測試再寫 code）

**如果 user 對任何設計決策有疑問**，先討論再開工。**別又像之前沒驗證先做**。

最後更新：2026-09-04 17:30
狀態：規劃中，等 user review
