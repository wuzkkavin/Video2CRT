# Handoff Notes - FUJII KAZE "Kirari" (TcLLpZBWsck)

**Created**: 2026-09-04
**Skill**: `video-crt-geom-libplacebo` v23 (commit 999086f)

## Source

- **URL**: https://www.youtube.com/watch?v=TcLLpZBWsck
- **Title**: FUJII KAZE - "Kirari" (MV)
- **Resolution**: 1920x1080 VP9, 4:00 (240s), SAR 1:1
- **Format**: 2.35:1 widescreen CONTENT (1920x820) inside 1920x1080 container — **上下各 130px 黑邊** (YouTube letterbox)
- **Audio**: AAC

## Final deliverable

| File | Size | Notes |
|---|---|---|
| `final.mp4` | 282MB | 4:00, 1920x1080, 16:9 滿版 (stretched) |
| `zh-Hant.srt` | 45 segments | YouTube CC source (gotcha 19) |
| `raw.mp4` | 724MB | libplacebo stretched 1920x820 -> 1920x1080 |
| `crt.glsl` | inherited | shared shader from yt_zsKjUez1xdU/ |
| `faster_whisper_out_en.json` | 20 segments | ASR fallback - kept for gotcha 20 cross-validation |
| `ytcc.en.srt` | 45 segments | raw YouTube CC SRT (source for zh-Hant.srt) |

## Edge case encountered (NEW GOTCHA-worthy)

### Source has letterbox (130px top + 130px bottom black bars)

`cropdetect` reported `crop=1920:820:0:130` consistently across 1500 frames.

User said "上下是黑邊的... 一樣滿版 1920x1080". This is the **first video in this project with vertical letterbox** (all previous 10 videos were 4:3 content in 16:9 container with side bars, or already full-frame).

### Decision: A. crop + stretch vertically

Three options were generated as preview frames:
- **A. crop 1920:820 then force_aspect=0 stretch to 1920x1080** (chosen)
- **B. leave as-is** (164s of black bars would be visible)
- **C. crop + scale uniform (preserve aspect, possible side crop)**

User chose A. Result: contents visually **stretched 32% vertically** (人物 characters appear taller than original). This was the user's explicit preference for "滿版" filling.

### filter expression used

```bash
-vf "crop=1920:820:0:130,libplacebo=custom_shader_path=crt.glsl:w=1920:h=1080:fps=30:force_original_aspect_ratio=0"
```

The `crop=1920:820:0:130` removes both letterbox strips; `force_original_aspect_ratio=0` then vertically stretches 820->1080 (32%).

### LESSON → becomes gotcha 24

For 2.35:1 widescreen source (e.g. MV/film crops with black bars top+bottom), the gotcha rule:
- detect vertical letterbox with cropdetect
- A: stretch to fill (user accepted this trade-off)
- B: leave bars (no transformation)
- C: zoom+uniform-scale (preserves aspect, may crop sides)

Document as `gotcha 24: handle vertical letterbox in source video`.

## Gotcha 20 cross-validation result

Both sources pulled:
- **YouTube CC**: 45 segments, clean English lyrics (Kirari by Fujii Kaze - 45 lines, matches official song)
- **Whisper medium.en**: 20 segments, all prob < 0.7, hallucinated: "Sorry" repeated, "I like a look it's a turn on our car whoo-dee-dee why he don't leaky", "你 deterioration 你消結起的哪哪哪..." etc.

**Decision**: CC wins by wide margin. Used CC for the SRT. Whisper output saved only as reference for future debugging.

## Subtitles on screen

Sample:
- 5s: Two made it by one, easily / 兩人輕易走過一場
- 60s: My eyes will always & forever be sparkling / 我的眼永遠閃閃發光
- 120s: Looking back on what I had to lose / 回想著我失去的
- 230s: black screen (song ends)

Verified at 5/60/120/180/230s.

## Edge case / lesson / new gotcha summary

1. **NEW: vertical letterbox source** → gotcha 24 documented above
2. **Whisper EN completely fails on Fuji Kaze MV** with chorus + instrumental → for high-energy Japanese MVs, default to medium.en + medium multi fallback → but ultimate fallback is YouTube CC, which has a robust lyrics database for popular songs
3. **CC SRT has multi-line text blocks** → need to flatten via `text = ' '.join(lines[2:])` + `re.sub(r'\s+', ' ', text)` before dictionary lookup
4. **whisper "Sorry" hallucination** repeated prob=0.01-0.13 → confirms gotcha 12 (medium not enough for noisy vocals, need CC)

## 跑過的 gotcha 列表

- ✅ gotcha 21 (install_skill.py 跑過)
- ✅ gotcha 22 (output dir 正確)
- ✅ gotcha 19 (YouTube CC 使用)
- ✅ gotcha 20 (CC vs Whisper cross-validate, CC wins)
- ✅ gotcha 17 (沒砍 weird ASR, 但 ASR fail 反正不影響 SRT)
- ❌ gotcha 16 (3+ min, but Whisper output quality 差, chunks 沒救)
- ❌ gotcha 18 (medium.en primary, 不是 fallback)
- ✅ cropdetect 自動偵測 vertical letterbox (新場景)

最後更新: 2026-09-04
