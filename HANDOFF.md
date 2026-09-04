# Video2CRT - HANDOFF

給未來對話或接手 agent 的完整指引。

## 你是新對話嗎？

讀這份檔開始：

1. **第一步（必做）：跑 install_skill.py 驗證環境與 skill**：
   ```bash
   cd "C:/Users/asaialabs/Documents/Hermes/Video2CRT"
   python scripts/install_skill.py
   ```
   **必須看到** `[ALL PASS] Skill v20 installed and dependencies OK.`，否則**不要**給用戶任何結論。
   若 `Found N gotchas` < 20 警告 → 該 skill 不是最新版，需先 `git -C ~/AppData/Local/hermes pull` 同步 GitHub

2. 看 [`README.md`](README.md) — 專案總覽
3. 看 [`docs/architecture.md`](docs/architecture.md) — 專案結構圖
4. 看 [`docs/index.md`](docs/index.md) — 11 支影片目錄 (output/ + archive/)
5. 看 [`docs/workflow.md`](docs/workflow.md) — 完整工作流程（從 URL 到 final.mp4）
6. 看 [`docs/recipes.md`](docs/recipes.md) — 場景食譜
7. 看 [`docs/troubleshooting.md`](docs/troubleshooting.md) — 常見錯誤排除
8. 看 [`CHANGELOG.md`](CHANGELOG.md) — 歷史紀錄 + 學到的 20 個 gotcha

如果用戶丟新 URL 給你：

1. **先跑** `python scripts/install_skill.py`，確保 [ALL PASS]
2. **工作目錄必須是**：`C:\Users\asaialabs\Documents\Hermes\Video2CRT\`（**絕對不要**放 Downloads/）
3. 跑 `docs/workflow.md` 的 Stage 1-11
4. 自動套用 skill `video-crt-geom-libplacebo` 的所有 gotcha（共 20 個）
5. **顯式宣告** `Found N gotchas in SKILL.md` 證明你已驗證
6. 跑 `python tests/run_all.py`（13 tests，應全綠）

## 工作目錄結構（這是關鍵）

```
C:\Users\asaialabs\Documents\Hermes\Video2CRT\
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

skill 本體：`C:\Users\asaialabs\AppData\Local\hermes\skills\video-crt-geom-libplacebo\SKILL.md`

GitHub mirror: `https://github.com/wuzkkavin/HermesFullSetup/blob/main/skills/video-crt-geom-libplacebo/SKILL.md`

最新版本：commit `695ca35`，含 20 個 gotcha。

### 20 個 gotcha 快速記憶

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

1. ❌ 砍 weird 文字像 Bullshit、Kick the hop (gotcha 17)
2. ❌ 用 avg_logprob 當獨立過濾 (gotcha 13)
3. ❌ 設 `initial_prompt=` 給 Whisper (gotcha 14)
4. ❌ 從 j-lyric.net / animesonglyrics.com / genius.com 抓歌詞 (gotcha 11)
5. ❌ 在 libplacebo shader 用 `//!HOOK RGB` (gotcha 1)
6. ❌ 把字幕跟 libplacebo 同一個 ffmpeg pass 跑 (gotcha 4)
7. ❌ mux 後忘記加 `-aspect 16:9` (gotcha 3)
8. ❌ 只看前 30 秒的字幕就說完成 (gotcha 10)
9. ❌ 把影片放 `Downloads/`（必須是 `output/yt_*/`）
10. ❌ 沒跑 `install_skill.py` 就開工

## 你接手時，**必須**：

1. ✅ **先跑 `python scripts/install_skill.py`** 並看到 [ALL PASS]
2. ✅ 工作目錄在 `Documents/Hermes/Video2CRT/output/yt_<id>/`
3. ✅ 用 faster-whisper **medium** 作為 primary ASR
4. ✅ 對 3+ 分鐘影片跑 chunked ASR (gotcha 16)
5. ✅ 對 multilingual 沉默 30+ 秒段跑 medium.en fallback (gotcha 18)
6. ✅ 視覺驗證**完整跨段**抽樣 (gotcha 10)
7. ✅ 字幕從 0:00 開始（如果 Whisper 從 0:00 給的話）
8. ✅ handoff.md 寫處理細節
9. ✅ `docs/index.md` / `CHANGELOG.md` / `README.md` 隨之更新
10. ✅ 任何新 gotcha 立即 commit + push 到 GitHub
11. ✅ 跑 `python tests/run_all.py`，13 tests 應全綠

---

最後更新：2026-09-04（v0.5.0 重新組織）
對話交接紀念：20 個 gotcha 已固化。專案結構：方案 C + MIT License。
