#!/usr/bin/env python3
"""Run faster-whisper ASR on a 16kHz wav file. Used by Video2CRT pipeline.

Stage 1 of the pipeline: extract ASR segments from <source_16k.wav>.
Stage 2: optional chunked ASR for long videos.
Stage 3: optional medium.en fallback for medium-multilingual silence.

Outputs <faster_whisper_out.json> in the current directory.

Usage:
    python run_whisper.py source_16k.wav                              # medium multilingual
    python run_whisper.py source_16k.wav --lang ja                   # explicit language
    python run_whisper.py source_16k.wav --model medium.en            # English model
    python run_whisper.py source_16k.wav --chunks 40                  # also do chunked (40s pieces)

Implements gotchas 12, 13, 14, 16, 18 from the video-crt-geom-libplacebo skill.
"""

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path


def extract_audio(video: Path) -> Path:
    """16kHz mono PCM for whisper. Returns path to source_16k.wav."""
    wav = video.parent / "source_16k.wav"
    if wav.exists():
        return wav
    print(f"Extracting audio from {video.name}...")
    r = subprocess.run(
        ["ffmpeg", "-y", "-i", str(video), "-vn", "-ar", "16000", "-ac", "1",
         "-c:a", "pcm_s16le", str(wav)],
        capture_output=True, text=True, timeout=300,
    )
    if r.returncode != 0:
        print(f"ffmpeg error: {r.stderr}", file=sys.stderr)
        sys.exit(1)
    return wav


def transcribe(model_name: str, audio: Path, language: str | None,
               beam_size: int = 10, compute_type: str = "int8"):
    """Load faster-whisper model and transcribe."""
    from faster_whisper import WhisperModel
    print(f"Loading model '{model_name}'...")
    model = WhisperModel(model_name, device="cpu", compute_type=compute_type)
    print(f"Transcribing {audio.name}...")
    kwargs = {
        "beam_size": beam_size,
        "word_timestamps": True,
        "vad_filter": False,
        "condition_on_previous_text": False,
        # NEVER set initial_prompt (gotcha 14)
    }
    if language:
        kwargs["language"] = language
    segments, info = model.transcribe(str(audio), **kwargs)
    print(f"Detected language: {info.language} (prob={info.language_probability:.2f})")
    results = []
    for seg in segments:
        text = seg.text.strip()
        if not text or text == "-":
            continue
        words_info = getattr(seg, "words", None) or []
        if words_info:
            avg_logprob = sum(w.probability for w in words_info if hasattr(w, "probability")) / max(len(words_info), 1)
        else:
            avg_logprob = 0.5
        duration = seg.end - seg.start
        chars_per_sec = len(text) / max(duration, 0.1)
        results.append({
            "start": seg.start,
            "end": seg.end,
            "text": text,
            "avg_logprob": avg_logprob,
            "chars_per_sec": chars_per_sec,
        })
        print(f"[{seg.start:6.2f}-{seg.end:6.2f}] prob={avg_logprob:.2f} chps={chars_per_sec:.0f} | {text!r}")
    return results


def chunked_transcribe(model_name: str, audio: Path, video: Path, language: str | None,
                       chunk_seconds: int = 40, beam_size: int = 10):
    """Per gotcha 16: chunked ASR for second-half gap-fill in 3+ min videos."""
    from faster_whisper import WhisperModel
    print(f"Chunked ASR ({chunk_seconds}s chunks)...")
    model = WhisperModel(model_name, device="cpu", compute_type="int8")
    duration_s = float(subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration",
         "-of", "default=nw=1", str(video)],
        capture_output=True, text=True, timeout=30,
    ).stdout.split("=")[1].strip())
    chunks = []
    for t in range(0, int(duration_s), chunk_seconds):
        clip = video.parent / f"clip_{t}.wav"
        subprocess.run(
            ["ffmpeg", "-y", "-i", str(video), "-ss", str(t), "-t", str(chunk_seconds),
             "-vn", "-ar", "16000", "-ac", "1", str(clip)],
            capture_output=True, timeout=30,
        )
        chunks.append((t, clip))
    results = []
    for offset, clip in chunks:
        kwargs = {
            "beam_size": beam_size,
            "word_timestamps": True,
            "vad_filter": False,
            "condition_on_previous_text": False,
        }
        if language:
            kwargs["language"] = language
        try:
            segments, _ = model.transcribe(str(clip), **kwargs)
            for seg in segments:
                text = seg.text.strip()
                if not text or text == "-":
                    continue
                words_info = getattr(seg, "words", None) or []
                avg_logprob = sum(w.probability for w in words_info if hasattr(w, "probability")) / max(len(words_info), 1) if words_info else 0.5
                chars_per_sec = len(text) / max(seg.end - seg.start, 0.1)
                results.append({
                    "start": offset + seg.start,
                    "end": offset + seg.end,
                    "text": text,
                    "avg_logprob": avg_logprob,
                    "chars_per_sec": chars_per_sec,
                })
        except Exception as e:
            print(f"  chunk {offset}-{offset+chunk_seconds}s failed: {e}")
    return results


def main():
    parser = argparse.ArgumentParser(description="Run Whisper ASR on a wav file.")
    parser.add_argument("audio", help="Path to source_16k.wav or video file")
    parser.add_argument("--video", help="Path to source.mp4 (for chunking)")
    parser.add_argument("--model", default="medium", help="Whisper model name (default: medium)")
    parser.add_argument("--lang", help="Language code (e.g., ja, en)")
    parser.add_argument("--chunks", type=int, help="Also run chunked ASR with N-second chunks (per gotcha 16)")
    parser.add_argument("--fallback-model", help="Optional fallback model (e.g., medium.en per gotcha 18)")
    parser.add_argument("--output", default="faster_whisper_out.json",
                        help="Output JSON path (default: faster_whisper_out.json)")
    parser.add_argument("--append", action="store_true",
                        help="Append to existing output JSON instead of overwriting")
    args = parser.parse_args()

    audio_path = Path(args.audio)
    if not audio_path.exists():
        print(f"Not found: {audio_path}", file=sys.stderr)
        sys.exit(1)

    # If .mp4 given, auto-extract audio
    if audio_path.suffix.lower() in (".mp4", ".mkv", ".webm"):
        video_path = audio_path
        audio_path = extract_audio(video_path)
    else:
        video_path = Path(args.video) if args.video else audio_path.parent / "source.mp4"

    # Stage 1: main ASR
    main_results = transcribe(args.model, audio_path, args.lang)

    # Stage 2 (optional): chunked ASR for gap-fill (gotcha 16)
    chunked_results = []
    if args.chunks and video_path.exists():
        try:
            chunked_results = chunked_transcribe(
                args.model, audio_path, video_path, args.lang,
                chunk_seconds=args.chunks,
            )
        except Exception as e:
            print(f"Chunked ASR failed: {e}", file=sys.stderr)

    # Stage 3 (optional): fallback model (e.g., medium.en per gotcha 18)
    fallback_results = []
    if args.fallback_model:
        try:
            fallback_results = transcribe(args.fallback_model, audio_path, args.lang)
        except Exception as e:
            print(f"Fallback ASR failed: {e}", file=sys.stderr)

    # Combine: main first, then chunks that don't overlap (gotcha 16), then fallback non-overlapping (gotcha 18)
    def overlaps(a, b):
        return not (a["end"] < b["start"] or a["start"] > b["end"])

    combined = list(main_results)
    used_ranges = [(s["start"], s["end"]) for s in main_results]
    for cs in chunked_results:
        if any(not (cs["end"] < u0 or cs["start"] > u1) for u0, u1 in used_ranges):
            continue
        combined.append(cs)
        used_ranges.append((cs["start"], cs["end"]))
    for fs in fallback_results:
        if any(not (fs["end"] < u0 or fs["start"] > u1) for u0, u1 in used_ranges):
            continue
        combined.append(fs)
        used_ranges.append((fs["start"], fs["end"]))

    combined.sort(key=lambda x: x["start"])

    # Write output
    output_path = Path(args.output)
    if args.append and output_path.exists():
        existing = json.loads(output_path.read_text(encoding="utf-8"))
        combined = existing + combined
        combined.sort(key=lambda x: x["start"])

    output_path.write_text(json.dumps(combined, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"\nTotal: {len(combined)} segments written to {output_path}")
    print(f"  - main ({args.model}): {len(main_results)}")
    print(f"  - chunked: {len(chunked_results)}")
    print(f"  - fallback ({args.fallback_model or 'none'}): {len(fallback_results)}")


if __name__ == "__main__":
    main()
