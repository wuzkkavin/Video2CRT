# Video2CRT Recipes

Common scenarios with the right pipeline choices.

## English song/video

Use `medium.en` as PRIMARY ASR (not fallback). Single-pass is usually enough.

```bash
python -m scripts.run_whisper source_16k.wav --model medium.en
# Stage 2 chunked optional
# Stage 3 fallback medium.en (rarely needed)
```

**Cross-validate with YouTube CC** (if video has CC):

```bash
yt-dlp -P <out_dir> --write-auto-subs --convert-subs srt \
       -o "source.%(ext)s" \
       "https://www.youtube.com/watch?v=..."
```

Compare per 5-second slice (gotcha 20):
- Match → use either
- Differ on song → Whisper wins
- Differ on dialogue → CC wins
- Both wrong → leave short phrase, don't guess

## Japanese live music

Use **medium multilingual** as primary + **chunked ASR** (gotcha 16) for second half + **medium.en** fallback (gotcha 18) for 30+ second silence gaps.

```bash
python scripts/run_whisper.py source_16k.wav \
       --model medium --lang ja \
       --chunks 40 \
       --fallback-model medium.en \
       --output output/yt_XYZ/faster_whisper_out.json
```

Note: small model hallucinates English idioms on Japanese vocals. NEVER use it.

## Movie clip / dialogue-heavy

Medium multilingual is good. Skip chunked if < 3 minutes. YouTube CC often reliable.

```bash
python scripts/run_whisper.py source_16k.wav --model medium
```

## Long video (6+ minutes)

ALWAYS enable chunked ASR to catch second-half accuracy degradation:

```bash
python scripts/run_whisper.py source_16k.wav --model medium --chunks 40
```

Use 0.5-1.0s gap threshold (not 0.3s) for 6+ minute videos to avoid libass fade ghost stacking.

## Source video has black bars (pillarbox/letterbox)

Run cropdetect FIRST before libplacebo:

```bash
ffmpeg -i source.mp4 -vf "cropdetect=24:2:0" \
       -frames:v 200 -f null - 2>&1 | grep -oE "crop=[0-9]+:[0-9]+:[0-9]+:[0-9]+" | tail -1
```

Use that crop in the raw render step (src/video2crt/pipeline.py render_raw(crop=...)).

## 4:3 source stretched to 16:9

Ensure `force_original_aspect_ratio=0` in libplacebo AND `-aspect 16:9` in final mux (gotcha 2, 3).

Otherwise player letterboxes 1920x1080 back to 4:3.

## Verify final output

```bash
# Extract frames at known segments
for t in 5 30 60 90 120; do
  ffmpeg -y -ss $t -i output/yt_XXX/final.mp4 \
         -frames:v 1 -vf "crop=1200:200:360:850" \
         -q:v 2 /tmp/v_t${t}.jpg
done
```

Sample at t=5, 25% mark, 50% mark, 75% mark, 95% mark (gotcha 10).
