# Video2CRT - 處理歷史 (Changelog)

2026-09-04 (台北標準時間, UTC+08:00) 整理。

## 重大事件

### 2026-09-04 — 項目重組
- **工作目錄搬移**：所有影片從 `C:\Users\<you>\Downloads\` 搬到 `C:\Users\<you>\Documents\Hermes\Video2CRT\`
- **建立專案文件**：README.md, HANDOFF.md, WORKFLOW.md, INDEX.md, CHANGELOG.md（本檔案）

### 2026-09-04 — Skill v20 (commit 695ca35)
- **20 個 gotcha** 完整記錄
- 新增 gotcha 19: YouTube auto-CC 允許 (用戶明示)
- 新增 gotcha 20: CC vs Whisper cross-validation (用戶明示)
- Push 到 GitHub: https://github.com/wuzkkavin/HermesFullSetup

## 11 支影片處理時間軸

### 2026-09-03 — 第一批（早）
| 時間 | 影片 | 結果 |
|---|---|---|
| - | yt_southern_cross (hJDbtViaL1Q) | ⚠️ partial — 跑了 mpv pipeline 後改 ffmpeg，srt 有但沒 final |
| - | yt_videoB | ❌ incomplete |

### 2026-09-04 — 第二批（黃昏前）
- **yt_city_hunter** — Sara: 第 1 個用「雙行不超過」字幕規則
- **yt_nat_king_cole** — When I Fall in Love: 第 1 個英文歌驗證
- **yt_new_video** — ストロンバー: 長影片 (4:31)
- **yt_new** — I Don't Know!: 長影片 (4:50)
- **yt_mr_usa** — Mr. USA v2
- **yt_test2** — 175R: 6:50 長影片，gotcha 9 段間距測試

### 2026-09-04 — 第三批（最終版 pipeline）
- **yt_zsKjUez1xdU** — SUPER MONKEY'S ミスターU.S.A. - Tour Edit
  - **重做 5 次**，每次學到新 gotcha
  - 從 18 段字幕砍到 13 段（錯），砍到 25 段（修），最後 36 段（正確）
  - gotcha 17 紀念日：「Bullshit」「Kick the hop」是真實 MC English 話語
  - gotcha 18 紀念日：medium.en 補滿 122-152s 段
- **yt_lesmiserables** — Do You Hear the People Sing?
  - **第一個用 medium.en 作為 PRIMARY 不是 fallback**
  - 54 段字幕, 跨 4:50 影片
  - gotcha 19 紀念日：用戶明示 YouTube CC 允許
- **yt_CaCSuzR4DwM** — partial, raw.mp4 跑了但沒 finish

## skill 演進（commit hash 對應）

| Commit | Gotcha | 學到的 |
|---|---|---|
| c15f5f6 | gotcha 14-16 | 不使用 initial_prompt / 不過度過濾 / 3 分鐘以上加 chunked |
| 9396589 | gotcha 17 REWORK gotcha 13 | 停止過度過濾「weird」ASR |
| 7966451 | gotcha 18 | medium.en fallback 補 multilingual 空白 |
| 8cc7b93 | gotcha 19 | YouTube CC 允許 |
| 695ca35 | gotcha 20 | CC vs Whisper cross-validation |
| 5376841 | refactor | reorganize whisper section |
| b989b52 | refactor | explicit stage 1/2/3 fallback chain |

## Gotcha 演化列表

| # | Gotcha | 起源 |
|---|---|---|
| 1 | //!HOOK MAIN (not RGB) | 學習 libplacebo shader hook 機制 |
| 2 | force_original_aspect_ratio=0 | 4:3 內容拉伸 |
| 3 | -aspect 16:9 mux | SAR/DAR 修正 |
| 4 | Two-step subtitle pipeline | libplacebo + 字幕不能在同一個 pass |
| 5 | GPU decode cuda/cuvid | AV1 來源需要 |
| 6 | cropdetect for 4:3 黑邊 | many YouTube uploads |
| 7 | faster-whisper (not openai) | openai FP32 太慢 |
| 8 | 兩行字幕，不要第三行 | 用戶明示（"Sara 第三行不用了"） |
| 9 | 0.3-0.5s gap 段間距 | libass fade-out ghost |
| 10 | 全段視覺驗證 | 漏了 2 分鐘段 |
| 11 | 不準網路找歌詞 | 用戶明示（誤刪 Bullshit 後再強調） |
| 12 | whisper medium (not small) | small model 嚴重幻覺 |
| 13 | 4-condition filter only | 過度過濾修 |
| 14 | NO initial_prompt | 會 prompt leakage |
| 15 | YT end-screen watermark filter | 字幕最後有「請訂閱」 |
| 16 | chunked ASR for 3+ min | medium 後段降準 |
| 17 | 不要砍 weird ASR | 用戶反覆投訴 |
| 18 | medium.en fallback | 122-152s 盲點 |
| 19 | YouTube CC 允許 | 用戶明示（"自動 CC 可以用"） |
| 20 | CC vs Whisper cross-validate | 用戶明示（"自動 CC 也可能是錯的"） |

## 待辦

- [ ] 補完 yt_southern_cross 的 final.mp4
- [ ] 重建 yt_videoB（缺 source.mp4）
- [ ] 補完 yt_CaCSuzR4DwM 的 final.mp4
- [ ] 9 支影片補寫 handoff.md（目前只有 yt_zsKjUez1xdU 和 yt_lesmiserables 有）
- [ ] 給每支影片的 handoff.md 加入 gotcha 編號參考
- [ ] 比較 final.mp4 品質，淘汰粗顆粒版本

---

最後更新：2026-09-04


### 2026-09-04 — 正式專案結構（方案 C + MIT）
新增 / 重組：
- **LICENSE** (MIT)
- **VERSION** (0.5.0)
- **.gitignore** + **.editorconfig**
- **Makefile** (選用)
- **docs/** 子目錄（architecture, workflow, recipes, troubleshooting, index）
- **src/video2crt/** Python package (subtitle, asr, pipeline modules)
- **tests/** 13 個 unittest (test_install_skill, test_whisper_stages, run_all.py)
- **output/** 統一的影片輸出目錄（8 支完成）
- **archive/** 不完整的（3 支）

詳細結構見 `docs/architecture.md`。
