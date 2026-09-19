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
    cloudTranslation    bool — use Rust translation between prepare/finalize.
                        Otherwise use the local multilingual model.
    translationModel    Cloud translation model id (informational, stored in
                        metadata; actual cloud pass is the Rust orchestrator's job)
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
import os
import subprocess
import sys
import traceback
from pathlib import Path
from typing import Any

# --- path bootstrap ----------------------------------------------------------
# Make `from video2crt.*` work both in the source tree and in the packaged
# runtime.  The installer sets VIDEO2CRT_APP_ROOT to runtime/app, so this
# process never depends on the developer's checkout.
_runtime_value = os.environ.get("VIDEO2CRT_RUNTIME_DIR", "").strip()
_RUNTIME_ROOT = Path(_runtime_value).resolve() if _runtime_value else Path(__file__).resolve().parents[1]
if not _RUNTIME_ROOT.is_dir():
    _RUNTIME_ROOT = Path(__file__).resolve().parents[1]
_REPO_ROOT = str(Path(os.environ.get("VIDEO2CRT_APP_ROOT", _RUNTIME_ROOT / "app")))
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
    is_bracketed_cue,
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
    # NVENC is fast on supported NVIDIA machines. Try the approved encoder
    # first, then retry with software encoding on ordinary PCs.
    video_codec = ["-c:v", "h264_nvenc", "-preset", "p4", "-cq", "23"]
    cmd = [
        "ffmpeg", "-y", "-i", "raw.mp4",
        "-vf", f"subtitles=zh-Hant.srt:force_style='{force_style}'",
        *video_codec,
        "-pix_fmt", "yuv420p", "-an", "subtitled.mp4",
    ]
    proc = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", errors="replace", cwd=str(cwd_dir), timeout=300)
    if proc.returncode != 0:
        fallback = [
            item for item in cmd
            if item not in ("h264_nvenc", "p4", "23", "-cq")
        ]
        codec_index = fallback.index("-c:v") + 1
        fallback[codec_index:codec_index + 1] = ["libx264"]
        preset_index = fallback.index("-preset") + 1
        fallback[preset_index:preset_index + 1] = ["veryfast"]
        pix_index = fallback.index("-pix_fmt")
        fallback[pix_index:pix_index] = ["-crf", "18"]
        proc = subprocess.run(fallback, capture_output=True, text=True,
                              encoding="utf-8", errors="replace",
                              cwd=str(cwd_dir), timeout=300)
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
    proc = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", errors="replace", cwd=str(cwd_dir), timeout=120)
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
    try:
        from faster_whisper import WhisperModel  # imported lazily to keep startup fast

        model = WhisperModel("medium", device="cpu", compute_type="int8")
    except Exception as exc:  # noqa: BLE001
        emit("asr", 0.0, f"chunked ASR model load failed (skipping gap-fill): {exc}")
        return []
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
                    "language": language or "en",
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
            "language": language or "en",
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


# Shared caption policy is used by both the app and verification scripts.
from subtitle_engine import (
    build_srt_original,
    build_srt_two_line,
    normalize_segments,
    split_spoken_captions,
    translate_locally,
)


def prefer_declared_caption_language(
    asr_segments: list[dict], asr_language: str | None, declared_language: str | None
) -> str | None:
    """Prefer metadata only when ASR's language result is demonstrably weak."""
    if not declared_language:
        return asr_language
    asr = (asr_language or "").lower().split("-")[0]
    declared = declared_language.lower().split("-")[0]
    if not asr or asr == declared:
        return declared
    confidence = max(
        (float(segment.get("language_probability", 0.0) or 0.0)
         for segment in asr_segments),
        default=0.0,
    )
    # Do not override a strong ASR result merely because upload metadata is
    # inaccurate. The reported Japanese video had 0.53 confidence but was
    # hallucinated as Russian, so 0.70 leaves a practical safety margin.
    return declared if confidence < 0.70 else asr_language


def finalize(args: dict[str, Any]) -> int:
    """Build and burn only after every required translation exists."""
    output_dir = Path(args["outputDir"])
    segments = json.loads((output_dir / "subtitle_segments.json").read_text(encoding="utf-8"))
    translation_file = output_dir / "subtitle_translations.json"
    translations = json.loads(translation_file.read_text(encoding="utf-8")) if translation_file.exists() else {}
    subtitle_mode = str(args.get("subtitleMode") or "bilingual")
    if subtitle_mode not in ("original", "bilingual"):
        raise ValueError(f"未知字幕模式：{subtitle_mode}")
    translation_mode = str(args.get("translationMode") or "local")
    if subtitle_mode == "bilingual" and translation_mode == "local":
        missing = [segment for segment in segments
                   if segment.get("language") != "zh" and not translations.get(segment["text"])]
        if missing:
            translations.update(translate_locally(
                missing, lambda message: emit("translate", 0.5, message)))
        # Segments the local model cannot translate are dropped from the
        # SRT entirely (neither translation nor original is shown), so one
        # bad cue never aborts the whole video.
        untranslated = [segment for segment in segments
                        if segment.get("language") != "zh" and not translations.get(segment["text"])]
        if untranslated:
            emit("translate", 0.5, f"略過 {len(untranslated)} 段無法翻譯（僅原文也不顯示）")
            segments = [segment for segment in segments
                        if segment.get("language") == "zh" or translations.get(segment["text"])]
        translation_file.write_text(
            json.dumps(translations, ensure_ascii=False, indent=2), encoding="utf-8")
    srt_text = (build_srt_original(segments) if subtitle_mode == "original"
                else build_srt_two_line(segments, translations))
    srt_path = output_dir / "zh-Hant.srt"
    srt_path.write_text(srt_text, encoding="utf-8")
    label = "僅原文" if subtitle_mode == "original" else "中文單行、其他語言雙行"
    emit("translate", 1.0, f"字幕已完成：{len(segments)} 段；{label}")
    source = output_dir / "source.mp4"
    raw = output_dir / "raw.mp4"
    final = output_dir / "final.mp4"
    subtitled = output_dir / "subtitled.mp4"
    if srt_text.strip():
        emit("burn", 0.0, "正在將完成的字幕燒入影片")
        burn_subtitles_local(raw, srt_path, subtitled, output_dir)
        emit("burn", 1.0, "字幕燒錄完成")
        video = subtitled
    else:
        emit("burn", 1.0, "未辨識到人聲，保留無字幕影片")
        video = raw
    emit("mux", 0.0, "正在封裝影片與原始聲音")
    mux_audio_local(video, source, final, output_dir)
    emit("mux", 1.0, "影片與聲音封裝完成")
    emit_done({"videoId": args.get("videoId", ""), "outputDir": str(output_dir),
               "finalMp4": str(final), "srt": str(srt_path)})
    return 0


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


def packaged_model_path(env_name: str, required_files: list[str]) -> str | None:
    """Use an installer-provided model directory when it is complete."""
    value = os.environ.get(env_name, "").strip()
    if not value:
        return None
    path = Path(value)
    if path.is_dir() and all((path / name).is_file() for name in required_files):
        return str(path)
    return None


def run(args: dict[str, Any]) -> int:
    """Run the four stages. Returns process exit code."""
    if args.get("phase") == "finalize":
        return finalize(args)
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

    emit("asr", 0.10, "載入本機 Whisper large-v3-turbo 語音辨識模型")
    from huggingface_hub import snapshot_download
    model_options = dict(repo_id="mobiuslabsgmbh/faster-whisper-large-v3-turbo",
        revision="0a363e9161cbc7ed1431c9597a8ceaf0c4f78fcf", token=False,
        allow_patterns=["config.json", "model.bin", "tokenizer.json", "vocabulary.txt"])
    model_path = packaged_model_path("VIDEO2CRT_ASR_MODEL_DIR", model_options["allow_patterns"])
    try:
        if model_path is None:
            model_path = snapshot_download(**model_options, local_files_only=True)
        if not all((Path(model_path) / name).is_file() for name in model_options["allow_patterns"]):
            raise FileNotFoundError("incomplete ASR model cache")
    except Exception:
        emit("asr", 0.10, "首次下載本機辨識模型（約 1.6 GB），影片不會上傳")
        model_path = snapshot_download(**model_options)
    main_segs = transcribe(audio, model_name=model_path, language=asr_language)
    main_out = output_dir / "faster_whisper_out.json"
    main_out.write_text(json.dumps(main_segs, ensure_ascii=False, indent=2), encoding="utf-8")
    emit("asr", 0.55, f"stage 1 complete: {len(main_segs)} segments")

    detected_language = main_segs[0].get("language", asr_language) if main_segs else asr_language
    # Local ASR is the ONLY subtitle source. YouTube caption tracks are
    # never fetched or adopted: live speeches and uncaptioned videos must
    # behave identically, and network caption availability must not be able
    # to change the output.
    # Do not infer "missing speech" from instrumental pauses and run an English
    # model over non-English songs. A single multilingual pass owns the timeline.
    duration = ffprobe_duration(source_mp4)
    segments = normalize_segments(split_spoken_captions(main_segs), duration)
    # Drop bracketed sound cues (e.g. ["Pomp and Circumstance"], [Music]):
    # non-speech markers the local translator must never see. Bare weird
    # speech (gotcha 17) is never bracket-wrapped and is always kept.
    segments = [s for s in segments if not is_bracketed_cue(s.get("text", ""))]
    subtitle_source = "local-asr"
    emit("asr", 0.9, f"採用本機語音辨識：{len(segments)} 段")
    (output_dir / "subtitle_segments.json").write_text(
        json.dumps(segments, ensure_ascii=False, indent=2), encoding="utf-8")
    (output_dir / "subtitle_source.json").write_text(json.dumps({
        "source": subtitle_source,
        "language": detected_language,
        "segments": len(segments),
    }, ensure_ascii=False, indent=2), encoding="utf-8")
    emit("asr", 1.0, f"字幕原文完成：{len(segments)} 段，語言 {detected_language or '未偵測'}")
    if args.get("phase") == "prepare":
        return 0
    return finalize(args)


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
