"""Whisper ASR integration with 3-stage fallback pipeline.

Implements gotchas 12, 13, 14, 16, 18 from video-crt-geom-libplacebo skill.
"""
from pathlib import Path
import subprocess


def extract_audio(video: Path) -> Path:
    """16kHz mono PCM wav for Whisper input."""
    wav = video.parent / "source_16k.wav"
    if wav.exists():
        return wav
    r = subprocess.run(
        ["ffmpeg", "-y", "-i", str(video), "-vn", "-ar", "16000", "-ac", "1",
         "-c:a", "pcm_s16le", str(wav)],
        capture_output=True, text=True, timeout=300,
    )
    if r.returncode != 0:
        raise RuntimeError(f"ffmpeg audio extract failed: {r.stderr}")
    return wav


def transcribe(audio: Path, model_name: str = "medium", language: str | None = None):
    """Stage 1: medium-multilingual Whisper ASR.

    Returns list of {start, end, text, avg_logprob, chars_per_sec}.
    """
    from faster_whisper import WhisperModel
    model = WhisperModel(model_name, device="cpu", compute_type="int8")
    kwargs = {
        "beam_size": 10,
        "word_timestamps": True,
        "vad_filter": False,
        "condition_on_previous_text": False,
        # NEVER initial_prompt (gotcha 14)
    }
    if language:
        kwargs["language"] = language
    segments, info = model.transcribe(str(audio), **kwargs)
    results = []
    for seg in segments:
        text = seg.text.strip()
        if not text or text == "-":
            continue
        words = getattr(seg, "words", None) or []
        avg_logprob = sum(w.probability for w in words if hasattr(w, "probability")) / max(len(words), 1) if words else 0.5
        duration = seg.end - seg.start
        results.append({
            "start": seg.start,
            "end": seg.end,
            "text": text,
            "avg_logprob": avg_logprob,
            "chars_per_sec": len(text) / max(duration, 0.1),
        })
    return results
