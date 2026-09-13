#!/usr/bin/env python3
"""Video2CRT Tauri sidecar: stages 4-6 (ASR + SRT + burn + mux).

This CLI is spawned by the Rust orchestrator as:

    python pipeline_cli.py '<json-args>'

Where `<json-args>` is a JSON object with the following keys:
    url                 YouTube URL (kept for traceability / handoff notes)
    outputDir           Per-video output directory (already contains source.mp4,
                        source_16k.wav and raw.mp4 from the Rust side)
    crop                Crop filter string (gotcha 6 + 24), passed through to
                        output metadata only — the Rust side already used it
                        for the raw.mp4 render
    asrLanguage         Optional Whisper language hint ("ja", "en", "zh", None)
    cloudTranslation    bool — if true, line 2 of SRT is left empty for the
                        cloud translation pass to fill in. If false, line 2
                        comes from `lyrics_translations.json` local lookup.
    translationModel    Cloud translation model id (informational, stored in
                        handoff.md; actual cloud pass is the React side's job)
    videoId             Deterministic video id from the URL

Sidecar stdout contract (one event per line, flushed):
    PROGRESS {"stage":"<name>","progress":<0..1>,"message":"<text>"}
    DONE     {"videoId":"...","outputDir":"...","finalMp4":"...","srt":"..."}
    ERROR    {"message":"..."}

Exit code 0 on success, non-zero on any failure.

Implements gotchas 7, 8, 9, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20 from the
`video-crt-geom-libplacebo` skill.
"""
from __future__ import annotations

import json
import subprocess
import sys
import traceback
from pathlib import Path
from typing import Any

# --- path bootstrap ----------------------------------------------------------
# Make `from video2crt.*` work whether the Tauri .exe ships us next to a
# bundled copy or alongside the live repo at C:/Users/asaialabs/Documents/
# Hermes/Video2CRT. The repo path is the canonical source (gotcha 22).
_REPO_ROOT = r"C:\Users\asaialabs\Documents\Hermes\Video2CRT"
_SRC_DIR = str(Path(_REPO_ROOT) / "src")

# Insert in priority order: src first (package root), then bare repo path so
# `python -m video2crt` style imports still resolve if src is missing.
for _p in (_SRC_DIR, _REPO_ROOT):
    if _p and _p not in sys.path:
        sys.path.insert(0, _p)

from video2crt.asr import extract_audio, transcribe  # noqa: E402
from video2crt.subtitle import (  # noqa: E402
    build_srt,
    fmt_time,
    is_skip,
    is_yt_watermark,
)
# NOTE: We deliberately do NOT import `burn_subtitles` / `mux_audio` from
# `video2crt.pipeline` because both inline absolute Windows paths into
# ffmpeg filter expressions, which breaks the libass filter parser on
# `C:\...` paths. Use the `*_local` wrappers below instead.


# --- local wrappers for burn/mux with cwd-relative paths --------------
# `video2crt.pipeline.burn_subtitles` inlines the SRT path into the
# libass filter expression (`subtitles=<path>:force_style=...`). When the
# path is a Windows absolute (`C:\Users\...`), the embedded colon breaks
# the filter parser — ffmpeg treats the path as the value of an unknown
# option ("original_size"). Same gotcha as Stage 3 (libplacebo shader
# path). Workaround: cd into output_dir and pass a plain relative name.
def burn_subtitles_local(raw: Path, srt: Path, subtitled: Path, cwd_dir: Path) -> Path:
    force_style = (
        "FontName=Microsoft JhengHei,FontSize=24,"
        "PrimaryColour=&H00FFFFFF,OutlineColour=&H00000000,"
        "BorderStyle=1,Outline=2,Shadow=0,MarginV=10"
    )
    cmd = [
        "ffmpeg", "-y", "-i", "raw.mp4",
        "-vf", f"subtitles=zh-Hant.srt:force_style='{force_style}'",
        "-c:v", "h264_nvenc", "-preset", "p4", "-cq", "23",
        "-pix_fmt", "yuv420p", "-an", "subtitled.mp4",
    ]
    proc = subprocess.run(cmd, capture_output=True, text=True, cwd=str(cwd_dir), timeout=300)
    if proc.returncode != 0:
        raise RuntimeError(f"subtitle burn failed: {proc.stderr}")
    return subtitled


def mux_audio_local(video_in: Path, source: Path, final: Path, cwd_dir: Path) -> Path:
    """Mux video from `video_in` (relative file name) with audio from `source`.

    `video_in` is usually the raw CRT-rendered mp4 or the subtitled
    burned-in mp4; this function doesn't care which — it just takes the
    first arg's video and the second arg's audio.

    Both inputs must already exist in `cwd_dir`. We pass them as plain
    file names (`raw.mp4` / `subtitled.mp4`) so libplacebo's `:` parser
    is never confused by a Windows absolute path. The caller is
    responsible for ensuring the file exists under cwd_dir before
    calling this function.
    """
    video_name = video_in.name
    source_name = source.name
    cmd = [
        "ffmpeg", "-y", "-i", video_name, "-i", source_name,
        "-map", "0:v", "-map", "1:a",
        "-c:v", "copy", "-c:a", "aac", "-b:a", "192k",
        "-aspect", "16:9", "-shortest", final.name,
    ]
    proc = subprocess.run(cmd, capture_output=True, text=True, cwd=str(cwd_dir), timeout=120)
    if proc.returncode != 0:
        raise RuntimeError(f"mux failed: {proc.stderr}")
    return final


# --- sidecar I/O helpers -----------------------------------------------------

def emit(stage: str, progress: float, message: str) -> None:
    """Emit one PROGRESS line and flush so the Rust parent reads it live."""
    payload = {"stage": stage, "progress": round(progress, 4), "message": message}
    sys.stdout.write("PROGRESS " + json.dumps(payload, ensure_ascii=False) + "\n")
    sys.stdout.flush()


def emit_done(payload: dict[str, str]) -> None:
    sys.stdout.write("DONE " + json.dumps(payload, ensure_ascii=False) + "\n")
    sys.stdout.flush()


def emit_error(message: str) -> None:
    sys.stdout.write("ERROR " + json.dumps({"message": message}, ensure_ascii=False) + "\n")
    sys.stdout.flush()


# --- video duration probe (used by gotcha 16 / 18 silence-gap detection) -----

def ffprobe_duration(video: Path) -> float:
    """Return source.mp4 duration in seconds via ffprobe. Returns 0.0 on error."""
    try:
        r = subprocess.run(
            [
                "ffprobe", "-v", "error",
                "-show_entries", "format=duration",
                "-of", "default=nw=1:nk=1",
                str(video),
            ],
            capture_output=True, text=True, timeout=30,
        )
        if r.returncode != 0:
            return 0.0
        return float(r.stdout.strip() or "0")
    except (ValueError, OSError, subprocess.TimeoutExpired):
        return 0.0


# --- stage 2 (gotcha 16): chunked ASR for second-half gap-fill ----------------

def chunked_transcribe(
    audio: Path,
    video: Path,
    language: str | None,
    chunk_seconds: int = 40,
    beam_size: int = 10,
) -> list[dict[str, Any]]:
    """Per gotcha 16: chunk the video into N-second clips, transcribe each
    independently with medium-multilingual (gotcha 7/12) and NO initial_prompt
    (gotcha 14). Returns a list of {start, end, text, ...} segments with
    absolute timestamps."""
    from faster_whisper import WhisperModel  # imported lazily to keep startup fast

    model = WhisperModel("medium", device="cpu", compute_type="int8")
    duration_s = ffprobe_duration(video)
    if duration_s <= 0:
        return []

    results: list[dict[str, Any]] = []
    chunks: list[tuple[float, Path]] = []
    for t in range(0, int(duration_s), chunk_seconds):
        clip = video.parent / f"clip_{t}.wav"
        # 16k mono per gotcha 12 input contract
        r = subprocess.run(
            [
                "ffmpeg", "-y", "-i", str(video),
                "-ss", str(t), "-t", str(chunk_seconds),
                "-vn", "-ar", "16000", "-ac", "1", str(clip),
            ],
            capture_output=True, timeout=60,
        )
        if r.returncode == 0 and clip.exists():
            chunks.append((float(t), clip))

    for offset, clip in chunks:
        kwargs: dict[str, Any] = {
            "beam_size": beam_size,
            "word_timestamps": True,
            "vad_filter": False,
            "condition_on_previous_text": False,
            # NEVER initial_prompt (gotcha 14)
        }
        if language:
            kwargs["language"] = language
        try:
            segments, _ = model.transcribe(str(clip), **kwargs)
            for seg in segments:
                text = seg.text.strip()
                if not text or text == "-":
                    continue
                words = getattr(seg, "words", None) or []
                avg_logprob = (
                    sum(w.probability for w in words if hasattr(w, "probability"))
                    / max(len(words), 1)
                    if words else 0.5
                )
                dur = seg.end - seg.start
                results.append({
                    "start": offset + seg.start,
                    "end": offset + seg.end,
                    "text": text,
                    "avg_logprob": avg_logprob,
                    "chars_per_sec": len(text) / max(dur, 0.1),
                })
        except Exception as exc:  # noqa: BLE001
            emit("asr", 0.0, f"chunked ASR chunk @ {offset:.0f}s failed: {exc}")

    return results


# --- silence-gap detection (drives gotcha 16 / 18 triggers) ------------------

def find_long_silence_gaps(segments: list[dict[str, Any]], min_gap_s: float = 30.0) -> bool:
    """Return True if main ASR left a >= min_gap_s silence gap. Used to decide
    whether to run chunked ASR (gotcha 16) and medium.en fallback (gotcha 18)."""
    if not segments:
        return True
    sorted_segs = sorted(segments, key=lambda s: s["start"])
    for prev, curr in zip(sorted_segs, sorted_segs[1:]):
        gap = curr["start"] - prev["end"]
        if gap >= min_gap_s:
            return True
    return False


# --- stage 4 (gotcha 18): medium.en fallback for multilingual silence --------

def medium_en_transcribe(
    audio: Path,
    language: str | None,
    beam_size: int = 10,
) -> list[dict[str, Any]]:
    """Per gotcha 18: medium.en fallback for 30+ second silence gaps in
    medium-multilingual output. Returns segments WITHOUT timestamps refinement
    for non-overlap merging — caller decides how to merge with main results."""
    from faster_whisper import WhisperModel

    model = WhisperModel("medium.en", device="cpu", compute_type="int8")
    kwargs: dict[str, Any] = {
        "beam_size": beam_size,
        "word_timestamps": True,
        "vad_filter": False,
        "condition_on_previous_text": False,
        # NO initial_prompt (gotcha 14)
    }
    if language:
        kwargs["language"] = language
    segments, _ = model.transcribe(str(audio), **kwargs)
    results: list[dict[str, Any]] = []
    for seg in segments:
        text = seg.text.strip()
        if not text or text == "-":
            continue
        words = getattr(seg, "words", None) or []
        avg_logprob = (
            sum(w.probability for w in words if hasattr(w, "probability"))
            / max(len(words), 1)
            if words else 0.5
        )
        dur = seg.end - seg.start
        results.append({
            "start": seg.start,
            "end": seg.end,
            "text": text,
            "avg_logprob": avg_logprob,
            "chars_per_sec": len(text) / max(dur, 0.1),
        })
    return results


# --- merge helper for chunked + medium.en non-overlap (gotcha 16 / 18) ------

def _overlaps(a: dict[str, Any], b: dict[str, Any]) -> bool:
    return not (a["end"] < b["start"] or a["start"] > b["end"])


def merge_segments(
    main: list[dict[str, Any]],
    extras: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    """Merge `extras` into `main`, keeping only extras that DO NOT overlap any
    main segment. Used for chunked ASR (gotcha 16) and medium.en fallback
    (gotcha 18) — both are gap-fills, never replacements."""
    combined = list(main)
    used = [(s["start"], s["end"]) for s in main]
    for s in extras:
        if any(not (s["end"] < u0 or s["start"] > u1) for u0, u1 in used):
            continue
        combined.append(s)
        used.append((s["start"], s["end"]))
    combined.sort(key=lambda x: x["start"])
    return combined


# --- custom SRT builder -------------------------------------------------------
# video2crt.subtitle.build_srt skips segments whose translation is missing.
# For cloud-translation mode (gotcha 8) we still want to emit every surviving
# segment with line 2 = "" so the cloud pass can fill it in. So we duplicate
# the build loop here with the optional "emit empty translations" flag.

def build_srt_two_line(
    segments: list[dict[str, Any]],
    translations: dict[str, str],
    *,
    emit_empty_translation: bool = False,
    end_margin_s: float = 0.5,
    gap_s: float = 0.3,
    min_duration_s: float = 1.5,
    min_sep_s: float = 0.5,
) -> str:
    """Two-line SRT (gotcha 8): line 1 = Whisper verbatim (gotcha 11 / 17),
    line 2 = translation (empty string when emit_empty_translation=True and
    translation is missing). Applies gotcha 9 gap control (two-pass forward /
    backward clip) and gotcha 13 / 15 filters via video2crt.subtitle.is_skip
    / is_yt_watermark."""
    # Pre-filter (gotcha 13 + 15). We do this before two-pass clip so the gap
    # math doesn't try to schedule text we will drop anyway.
    kept: list[dict[str, Any]] = []
    for s in segments:
        text = s["text"]
        if is_skip(text):  # gotcha 13: 4-condition filter
            continue
        if is_yt_watermark(text):  # gotcha 15: end-screen watermark
            continue
        # gotcha 17: keep "weird" text — only the 4-condition filter drops.
        # No avg_logprob gate here (also gotcha 13).
        kept.append(s)

    if not kept:
        return ""

    # --- forward pass: shift start_i forward so each subtitle starts at least
    # `gap_s` after the previous end (and at least `min_sep_s` later for the
    # libass fade-out safety margin). See gotcha 9.
    prev_end = 0.0
    forward: list[dict[str, Any]] = []
    for s in kept:
        ns = max(s["start"] + gap_s, prev_end + min_sep_s)
        forward.append({**s, "_start": ns, "_end": s["end"]})
        prev_end = ns

    # --- backward pass: clip end_i so it doesn't bleed into the next subtitle
    # start (gotcha 9: clip BOTH ends, not just shift start). Floor at
    # start_i + min_duration_s so single-line segments stay readable.
    backward: list[dict[str, Any]] = list(forward)
    for i in range(len(backward) - 2, -1, -1):
        next_start = backward[i + 1]["_start"]
        clipped_end = min(backward[i]["_end"], next_start - gap_s)
        backward[i]["_end"] = max(clipped_end, backward[i]["_start"] + min_duration_s)

    # --- format
    lines: list[str] = []
    for i, s in enumerate(backward, start=1):
        zh = translations.get(s["text"], "")
        if not zh and not emit_empty_translation:
            # No translation available and not in cloud-fill mode — skip this
            # entry. Per gotcha 11 we must NOT substitute web-fetched lyrics.
            continue
        lines.append(str(i))
        lines.append(f"{fmt_time(s['_start'])} --> {fmt_time(s['_end'])}")
        lines.append(s["text"])  # gotcha 8: line 1 = ASR verbatim
        lines.append(zh)         # gotcha 8: line 2 = translation (or empty)
        lines.append("")
    return "\n".join(lines)


# --- main pipeline ------------------------------------------------------------

def load_local_translations(repo_root: Path) -> dict[str, str]:
    """Load `lyrics_translations.json` (per-video curated Trad. Chinese dict)
    for offline translation fallback when cloudTranslation=false."""
    p = repo_root / "lyrics_translations.json"
    if not p.exists():
        return {}
    try:
        return json.loads(p.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}


def run(args: dict[str, Any]) -> int:
    """Run the four stages. Returns process exit code."""
    url = args.get("url", "")
    output_dir = Path(args["outputDir"])
    crop = args.get("crop", "") or ""
    # Normalise the ASR language hint. OptionsPage exposes
    # "auto" / "ja" / "en" / "zh" but faster_whisper rejects the
    # literal "auto" (it expects None for autodetect or a real
    # ISO-639-1 code like "ja"). Map "auto" → None so Whisper
    # auto-detects the language.
    raw_asr = args.get("asrLanguage")
    asr_language: str | None
    if raw_asr is None or raw_asr == "" or raw_asr == "auto":
        asr_language = None
    else:
        asr_language = raw_asr
    cloud_translation = bool(args.get("cloudTranslation", False))
    translation_model = args.get("translationModel") or ""
    video_id = args.get("videoId") or Path(output_dir).name

    if not output_dir.exists():
        emit_error(f"outputDir does not exist: {output_dir}")
        return 2

    source_mp4 = output_dir / "source.mp4"
    raw_mp4 = output_dir / "raw.mp4"
    if not source_mp4.exists():
        emit_error(f"source.mp4 not found in {output_dir} (Rust side must run download stage first)")
        return 2
    if not raw_mp4.exists():
        emit_error(f"raw.mp4 not found in {output_dir} (Rust side must run render stage first)")
        return 2

    repo_root = Path(_REPO_ROOT)
    local_translations = load_local_translations(repo_root)

    # ---- Stage 4 ASR (gotcha 7, 12, 14, 16, 18) ----------------------------
    emit("asr", 0.05, "extracting 16kHz mono audio for Whisper")
    audio = extract_audio(source_mp4)

    emit("asr", 0.10, "stage 1/3: faster-whisper medium-multilingual (int8)")
    main_segs = transcribe(audio, model_name="medium", language=asr_language)
    main_out = output_dir / "faster_whisper_out.json"
    main_out.write_text(json.dumps(main_segs, ensure_ascii=False, indent=2), encoding="utf-8")
    emit("asr", 0.55, f"stage 1 complete: {len(main_segs)} segments")

    # Decide whether to run chunked ASR (gotcha 16) and/or medium.en (gotcha 18)
    has_silence_gap = find_long_silence_gaps(main_segs, min_gap_s=30.0)

    chunk_segs: list[dict[str, Any]] = []
    en_segs: list[dict[str, Any]] = []

    if has_silence_gap:
        emit("asr", 0.60, "30s+ silence gap detected — running chunked ASR (gotcha 16)")
        try:
            chunk_segs = chunked_transcribe(audio, source_mp4, asr_language)
            chunk_out = output_dir / "faster_whisper_out_chunks.json"
            chunk_out.write_text(json.dumps(chunk_segs, ensure_ascii=False, indent=2), encoding="utf-8")
        except Exception as exc:  # noqa: BLE001
            emit("asr", 0.65, f"chunked ASR failed: {exc}")
        emit("asr", 0.70, f"chunked ASR complete: {len(chunk_segs)} raw segments")

        # After merging chunked, re-check for remaining silence (gotcha 18).
        merged_after_chunks = merge_segments(main_segs, chunk_segs)
        if find_long_silence_gaps(merged_after_chunks, min_gap_s=30.0):
            emit("asr", 0.75, "still silent — running medium.en fallback (gotcha 18)")
            try:
                en_segs = medium_en_transcribe(audio, asr_language)
                en_out = output_dir / "faster_whisper_out_en.json"
                en_out.write_text(json.dumps(en_segs, ensure_ascii=False, indent=2), encoding="utf-8")
            except Exception as exc:  # noqa: BLE001
                emit("asr", 0.78, f"medium.en fallback failed: {exc}")
            emit("asr", 0.82, f"medium.en fallback complete: {len(en_segs)} raw segments")
        else:
            emit("asr", 0.82, "chunked ASR closed the silence gaps — medium.en not needed")
    else:
        emit("asr", 0.82, "no 30s+ silence gaps — chunked / medium.en skipped")

    merged_segs = merge_segments(main_segs, chunk_segs)
    merged_segs = merge_segments(merged_segs, en_segs)
    emit("asr", 0.90, f"merged ASR: {len(merged_segs)} segments (main={len(main_segs)}, chunks={len(chunk_segs)}, en={len(en_segs)})")

    # ---- Stage 5 SRT build (gotcha 8, 9, 11, 13, 15, 17) ------------------
    emit("srt", 0.10, "building bilingual SRT (gotcha 8: two-line, gotcha 11: ASR-only)")
    # Choose translation source. In cloud mode line 2 starts empty; in local
    # mode we look up `lyrics_translations.json` for any matching line.
    translations_for_build: dict[str, str] = (
        {} if cloud_translation else dict(local_translations)
    )
    srt_text = build_srt_two_line(
        merged_segs,
        translations_for_build,
        emit_empty_translation=cloud_translation,
    )
    srt_path = output_dir / "zh-Hant.srt"
    srt_path.write_text(srt_text, encoding="utf-8")
    blocks = [b for b in srt_text.split("\n\n") if b.strip()]
    emit("srt", 0.50, f"SRT written: {len(blocks)} entries to {srt_path.name}")

    # ---- Stage 6 burn + mux (gotcha 4: subtitles separate from libplacebo) -
    subtitled_mp4 = output_dir / "subtitled.mp4"
    if blocks:
        emit("burn", 0.05, "burning subtitles onto raw.mp4 (h264_nvenc, libass)")
        burn_subtitles_local(raw_mp4, srt_path, subtitled_mp4, output_dir)
        emit("burn", 0.55, "subtitle burn complete")
        emit("mux", 0.05, "muxing audio with -aspect 16:9 (gotcha 3)")
        final_mp4 = output_dir / "final.mp4"
        mux_audio_local(subtitled_mp4, source_mp4, final_mp4, output_dir)
        emit("mux", 0.55, "mux complete")
    else:
        # Empty SRT (no translatable ASR survived filters, or local dict has no
        # matches and cloud translation is off). Skip burn/mux and use raw.mp4
        # as the final output — we still want a deliverable.
        emit("burn", 0.05, "no SRT entries to burn; using raw.mp4 as final")
        final_mp4 = output_dir / "final.mp4"
        # Best-effort: mux audio into raw.mp4 with -aspect 16:9.
        try:
            mux_audio_local(raw_mp4, source_mp4, final_mp4, output_dir)
        except RuntimeError:
            # If mux also fails (e.g. no audio in source), fall back to copy.
            import shutil as _sh
            _sh.copyfile(raw_mp4, final_mp4)
        emit("mux", 0.55, "mux complete (raw fallback)")

    # ---- Hand-off note for the React UI ----------------------------------
    handoff_path = output_dir / "handoff.md"
    if not handoff_path.exists():
        handoff_path.write_text(
            "\n".join([
                f"# handoff — {video_id}",
                "",
                f"- url: {url}",
                f"- crop: {crop or '(default 960:720:160:0 per gotcha 6/24)'}",
                f"- asr_language: {asr_language or 'auto'}",
                f"- cloud_translation: {cloud_translation}",
                f"- translation_model: {translation_model or '(n/a)'}",
                "",
                "## ASR counts",
                f"- medium (main): {len(main_segs)}",
                f"- chunked:       {len(chunk_segs)}",
                f"- medium.en:     {len(en_segs)}",
                f"- merged:        {len(merged_segs)}",
                "",
                "## gotchas applied",
                "7, 8, 9, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20",
                "",
            ]),
            encoding="utf-8",
        )

    emit_done({
        "videoId": video_id,
        "outputDir": str(output_dir),
        "finalMp4": str(final_mp4),
        "srt": str(srt_path),
        "subtitledMp4": str(subtitled_mp4),
    })
    return 0


def main() -> int:
    if len(sys.argv) < 2:
        emit_error("usage: pipeline_cli.py '<json-args>'")
        return 2
    try:
        args = json.loads(sys.argv[1])
    except json.JSONDecodeError as exc:
        emit_error(f"invalid JSON args: {exc}")
        return 2
    try:
        return run(args)
    except SystemExit as e:
        code = int(e.code) if isinstance(e.code, int) else 1
        emit_error(f"pipeline aborted with exit code {code}")
        return code
    except Exception as exc:  # noqa: BLE001
        # Last-resort: surface full traceback to stderr (debug) and a JSON
        # error line to stdout (sidecar contract).
        traceback.print_exc()
        emit_error(f"{type(exc).__name__}: {exc}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
