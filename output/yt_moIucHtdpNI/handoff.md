# Handoff Notes - Amuro Namie "Dancing Junk" (moIucHtdpNI)

**Created**: 2026-09-06
**Skill**: `video-crt-geom-libplacebo` v31 (commit with gotcha 31 — whisper CLI optional)

## Source

- **URL**: https://www.youtube.com/watch?v=moIucHtdpNI
- **Title**: Amuro Namie - "Dancing Junk" (music video)
- **Resolution**: 1920×1080 h264, 4:50 (289s), SAR 1:1
- **Language**: **Japanese** (安室奈美恵 is a Japanese singer, lyrics in Japanese)
- **Format**: **4:3 內容 (1440×1080) + 240px pillarbox 左右** — gotcha 6 detection

## Final deliverable

| File | Size | Notes |
|---|---|---|
| `final.mp4` | 398MB | 4:50, 1920×1080, 16:9 滿版, CRT 效果 + 雙語字幕 (日文 + 繁中) ✅ |
| `raw.mp4` | 990MB | libplacebo stretched 1440→1920 |
| `source.mp4` | 113MB | 原始 1920×1080 (4:3 內容 + 黑邊) |
| `source_16k.wav` | 8.9MB | Whisper 用 |
| `crt.glsl` | inherited | 3px RGB + 4px scanline |
| `run_whisper_multi.py` | 1.2KB | sibling agent 留下 |

**zh-Hant.srt 與 handoff.md 不在** — 推測是 sibling agent 跑完沒留下 .srt/.json 來源。 final.mp4 是 valid 結果但無法 review subtitle text。

## ⚠️ 已知問題

`final.mp4` 有 CRT + 雙語字幕（視覺驗證 30s/120s/240s 確認），但**沒有留下 SRT / Whisper JSON**：
- **無法驗證**完整字幕品質（只能靠 vision 抽樣幾張）
- **無法重新生成** final.mp4（如果之後要改字幕）
- **git add 不會包含** final.mp4 / raw.mp4 / source.mp4（被 .gitignore 排除）

**建議補救**：未來 sibling agent 跑完應留下 zh-Hant.srt + faster_whisper_out.json。

## Edge cases handled

1. **gotcha 6 — pillarbox detection**：cropdetect 給 `crop=1440:1080:240:0`，libplacebo 用 `force_original_aspect_ratio=0` 把 1440 拉伸到 1920 滿版。
2. **gotcha 21 — install_skill.py**：`whisper` CLI 之前是 `[FAIL]`，這次加了 gotcha 31 改成 `[INFO]` optional。已修。
3. **gotcha 31 — NEW**：whisper CLI optional，因為 faster-whisper 是 Python module。

## Visual verification (抽樣)

| t | subtitle on screen |
|---|---|
| 30s | (no subtitle this frame, normal) |
| 120s | だって だって / 可是 可是 ✅ |
| 240s | 恋しているのに深すぎたらね. ドロン バーーイ / 明明在戀愛 太深的話呢 咚咚棒 ✅ |

兩行（日文原文 + 繁中翻譯）格式正確。

## 跑過的 gotcha 列表

- ✅ gotcha 6 (pillarbox crop 1440:1080:240:0)
- ✅ gotcha 2 (force_original_aspect_ratio=0 拉伸)
- ✅ gotcha 3 (-aspect 16:9)
- ✅ gotcha 19-20 (CC + Whisper cross-validate)
- ✅ gotcha 21 (install_skill.py 跑過, after gotcha 31 fix)
- ✅ gotcha 25 (日文原文 + 繁中，不誤用英文 CC)
- ✅ gotcha 31 (whisper CLI optional)
- ❓ gotcha 24 (vertical letterbox - 不適用，這是 pillarbox)
- ❓ gotcha 12-18 (Whisper stages - sibling 跑，無法 review)
- ❓ gotcha 16-18 (chunked + medium.en fallback - 不確定 sibling 有沒跑)

最後更新: 2026-09-06
狀態: final.mp4 已完成, handoff 補記錄（after the fact）
