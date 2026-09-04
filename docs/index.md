# Video2CRT - 影片目錄

11 支影片處理記錄。每支影片的細節在 `yt_<id>/handoff.md`。

## 完整列表

| # | 影片 ID 資料匣 | 標題 | 時長 | final.mp4 | SRT 段數 | 狀態 |
|---|---|---|---|---|---|---|
| 1 | `yt_zsKjUez1xdU` | SUPER MONKEY'S - ミスターU.S.A. (Tour Edit) | 3:09 | ✅ 203MB | 36 | ✅ 完成 |
| 2 | `yt_southern_cross` | 稲垣潤一 - サザンクロス | 3:53 | ❌ raw only | 16 | ⚠️ 缺 final |
| 3 | `yt_videoB` | (mystery B) | - | ❌ 缺 source | - | ❌ 損毀/未完成 |
| 4 | `yt_new` | (new) | 4:50 | ✅ 397MB | 55 | ✅ 完成 |
| 5 | `yt_new_video` | (new video) | 4:31 | ✅ 315MB | 64 | ✅ 完成 |
| 6 | `yt_test2` | (test 2 - 175R) | 6:50 | ✅ 339MB | 52 | ✅ 完成 |
| 7 | `yt_mr_usa` | (Mr. USA v2) | 3:10 | ✅ 243MB | 31 | ✅ 完成 |
| 8 | `yt_nat_king_cole` | Nat King Cole - When I Fall in Love | 2:56 | ✅ 121MB | 9 | ✅ 完成 |
| 9 | `yt_city_hunter` | Sara (City Hunter 2) | 1:31 | ✅ 118MB | 24 | ✅ 完成 |
| 10 | `yt_lesmiserables` | Do You Hear the People Sing? (Les Misérables) | 4:50 | ✅ 379MB | 54 | ✅ 完成 |
| 11 | `yt_CaCSuzR4DwM` | (CaCSuzR4DwM) | 2:25 | ❌ raw only | - | ⚠️ 缺 final |

**狀態**:
- ✅ 完成 = final.mp4 + SRT + 字幕檢查
- ⚠️ 缺 final = 跑了 raw 但沒燒字幕，需 finish
- ❌ 損毀 = 缺 source.mp4，需要重新下載

## 按處理時間排序（最新在上）

1. **yt_lesmiserables** (2026-09-04): 第一個用 medium.en 的英文劇院曲
2. **yt_CaCSuzR4DwM** (2026-09-04): 短影片，raw 跑完但沒繼續
3. **yt_zsKjUez1xdU** (2026-09-04): 學到超多 — hallucination、gotcha 17 → 20 都在這部誕生
4. **yt_nat_king_cole** (2026-09-04): 英文歌驗證
5. **yt_test2** (2026-09-04): 6:50 長影片，10+ 字幕段，段間距測試
6. **yt_new_video** (2026-09-04): 長影片
7. **yt_new** (2026-09-04): 同上
8. **yt_mr_usa** (2026-09-04): SUPER MONKEY'S 第二版
9. **yt_city_hunter** (2026-09-04): Sara - 第一個用「雙行不要第三行」規則
10. **yt_videoB** (2026-09-03): 早 — incomplete
11. **yt_southern_cross** (2026-09-03): サザンクロス — 第一個用 mpv pipeline 之後改 ffmpeg libplacebo

## 每支影片的特性

- **`yt_zsKjUez1xdU`** — SUPER MONKEY'S - ミスターU.S.A. - Tour Edit
  - 1440×1080 AV1, 3:09, 純日文 live 演唱會
  - **重要發現**：small 模型幻覺嚴重 (Forever / Do you remember) → 改 medium
  - **重要發現**：medium-multilingual 對後段 122-152s 漏偵測 → 改 medium.en fallback

- **`yt_lesmiserables`** — Do You Hear the People Sing? (Les Misérables)
  - 1920×1080 AV1, 4:50, 英文音樂劇
  - 有 YouTube 自動 CC (但用 medium.en 不依賴 CC)
  - 54 段字幕，涵蓋歌曲 + 對話 + 第二首歌 reprise

- **`yt_test2`** — 175R (?) 6:50 長影片
  - 52 段字幕
  - **驗證 gotcha 9**：0.5-1.0s gap 是必要的

- **`yt_city_hunter`** — Sara
  - 24 段字幕
  - **驗證 gotcha 8**：第三行（括號註釋）會混亂，刪掉

- **`yt_nat_king_cole`** — When I Fall in Love (Nat King Cole)
  - 9 段字幕, 最短
  - 英文歌 (`.en` 模型)

- **`yt_new_video`, `yt_new`** — 較長影片 (4:30+)
  - 64 / 55 段字幕
  - Whisper medium 多語言模式

- **`yt_southern_cross`** — サザンクロス
  - 16 段字幕 (source.srt 是 yt-dlp 抽的官方 CC - 注意這是 Aladdin 自動 CC, 不是官方歌詞)
  - **缺 final.mp4**: 跑了 pipeline 但未燒字幕

- **`yt_videoB`** — 早/不完整
  - 沒 source.mp4，可能需要重新下載

- **`yt_CaCSuzR4DwM`** — partial
  - 跑了 raw.mp4 但沒繼續

- **`yt_mr_usa`** — SUPER MONKEY'S 第二版本
  - 31 段字幕
  - 用 medium 多語言

## 每支影片的 handoff.md 狀態

只有 `yt_lesmiserables` 和 `yt_zsKjUez1xdU` 有完整 handoff.md。
其他 9 支缺 — 之後有空或需要時補上。

---

最後更新：2026-09-04
