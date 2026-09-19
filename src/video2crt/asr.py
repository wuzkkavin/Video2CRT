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
    # Prefer the downloaded model; a network metadata timeout must not stall ASR.
    # Single-threaded inference: multi-threaded beam search is nondeterministic
    # across runs (same audio must yield same segments for reproducible output).
    try:
        model = WhisperModel(model_name, device="cpu", compute_type="int8", local_files_only=True,
                             cpu_threads=1, num_workers=1)
    except (OSError, ValueError):
        model = WhisperModel(model_name, device="cpu", compute_type="int8",
                             cpu_threads=1, num_workers=1)
    kwargs = {
        "beam_size": 10,
        "word_timestamps": True,
        "vad_filter": False,
        "condition_on_previous_text": False,
        # NEVER initial_prompt (gotcha 14)
        # Scalar temperature 0 disables faster-whisper's fallback ladder
        # ([0.0, 0.2, ...] resamples bad windows with RANDOM sampling, so
        # the same audio yields different segments every run). Deterministic
        # output is required for reproducible subtitles.
        "temperature": 0.0,
    }
    if language:
        kwargs["language"] = language
    def collect(word_timestamps: bool):
        """Consume Whisper's lazy iterator while its exceptions are catchable.

        Some versions of faster-whisper occasionally raise an IndexError from
        word alignment on music videos (an empty timestamp array is indexed by
        a boolean mask). Segment timestamps are still usable, so retrying
        without word alignment gives the user subtitles instead of aborting the
        whole conversion.
        """
        attempt = {**kwargs, "word_timestamps": word_timestamps}
        segments, info = model.transcribe(str(audio), **attempt)
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
                "language": info.language,
                "language_probability": info.language_probability,
                "avg_logprob": seg.avg_logprob,
                "no_speech_prob": seg.no_speech_prob,
                "words": [{"start": w.start, "end": w.end, "word": w.word} for w in words],
                "chars_per_sec": len(text) / max(duration, 0.1),
            })
        return results

    try:
        return collect(word_timestamps=True)
    except IndexError as error:
        # Do not broadly hide ASR failures. This is the known empty-alignment
        # failure; retry only when it is the failing word-timestamp operation.
        if "boolean index did not match" not in str(error):
            raise
        return collect(word_timestamps=False)
