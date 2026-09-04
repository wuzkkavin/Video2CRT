# Video2CRT Troubleshooting

## Symptom: subtitle shows "請訂閱本頻道" or similar at end

**Cause**: YouTube end-screen watermark text leaked into ASR. Gotcha 15 should filter.

**Fix**: Verify zh-Hant.srt doesn't contain チャンネル登録 / subscribe / サブタイトル keywords. If it does, the SRT build script wasn't using is_yt_watermark() — re-run it.

## Symptom: subtitles have only English / Chinese but no JP/JP etc on a JP song

**Cause**: medium-multilingual missed the second-half (gotcha 16) or your SRT skipped segments (gotcha 17 over-filter).

**Fix**: 
1. Run chunked ASR with `--chunks 40`
2. Run medium.en fallback with `--fallback-model medium.en` if there's a 30+ second silence gap
3. Verify your 4-condition filter doesn't filter out `avg_logprob < 0.3` segments (gotcha 13, 17)

## Symptom: subtitle text is `<junk> ` repeated 30 times

**Cause**: Whisper initial_prompt leakage (gotcha 14). You set `initial_prompt='日本語...'` somewhere.

**Fix**: Remove `initial_prompt=` entirely. Medium multilingual detects language without prompt bias.

## Symptom: 1920x1080 video shows letterboxed to 4:3 in player

**Cause**: missing `-aspect 16:9` in final mux (gotcha 3) OR missing `force_original_aspect_ratio=0` in libplacebo (gotcha 2).

**Fix**:
- libplacebo step: ensure `force_original_aspect_ratio=0` is in the filter string
- Final mux step: ensure `-aspect 16:9` is passed to ffmpeg
- Verify with `ffprobe`: should show `sample_aspect_ratio=1:1, display_aspect_ratio=16:9`

## Symptom: green tint or color shifted

**Cause**: blend filter in YUV420p domain causing color subsampling, OR RGB shader not in RGB domain.

**Fix**: libplacebo renders in RGB domain — should be clean. If green, the shader is wrong. Check it has `vec4 hook()` returning RGB.

## Symptom: CRT effect not visible

**Cause**: Wrong shader hook (gotcha 1) OR file extension not `.glsl`.

**Fix**:
- Shader must start with `//!HOOK MAIN` (NOT `//!HOOK RGB`)
- File must end in `.glsl` (not `.hook`, not `.frag`)
- Verify with: `ffmpeg ... -vf "libplacebo=custom_shader_path=crt.glsl:..."` works without "no shader function found" error

## Symptom: subtitle shows 3+ lines stacked (ghost)

**Cause**: segments too close together (libass fade residual, gotcha 9).

**Fix**: increase gap_s in build_srt() to 0.5-1.0s. Built default is 0.3s end-margin + 0.5s gap.

## Symptom: env says "Found X gotchas but X < 20"

**Cause**: Hermes skill out of date.

**Fix**:
```bash
cd /c/Users/asaialabs/AppData/Local/hermes
git pull origin main
```

## Symptom: install_skill.py reports "ffmpeg missing" but ffmpeg.exe exists

**Cause**: ffmpeg.exe at WinGet path `C:/Users/asaialabs/AppData/Local/Microsoft/WinGet/Links/ffmpeg.exe` not on git-bash PATH.

**Fix**: The script handles this path. If still failing, run:
```bash
python scripts/install_skill.py
```
and look for the ffmpeg stdout/stderr extraction. If still missing, add your ffmpeg path to `install_skill.py` `candidates["ffmpeg"]` list.

## Symptom: ASR takes forever

**Cause**: large model on CPU.

**Fix**: stick with `medium` int8 (~3 min for 3-min audio). `large-v2` takes ~30 min for same audio. GPU faster but requires CUDA setup.

## Symptom: subtitles appear in wrong order / missing after position X

**Cause**: Either Whisper 30+ second silence (gotcha 18 trigger) OR chunked ASR not enabled.

**Fix**: Check time gaps in zh-Hant.srt. If 30+ second gap with no entries, that's a Whisper silence. Run medium.en fallback on that section.

## Symptom: source video is 4:3 with side black bars (pillarbox)

**Cause**: YouTube uploads 4:3 content in a 16:9 container.

**Fix**: run cropdetect and apply crop filter before libplacebo step. See recipes.md "Source video has black bars".

## Symptom: medium.en produced all-English for Japanese song

**Cause**: That's expected. Medium.en romanizes Japanese lyrics to English phonetics.

**Fix**: use medium multilingual as PRIMARY, medium.en only as FALLBACK for silence gaps.
