from faster_whisper import WhisperModel
import json
import sys

print("Loading model...", flush=True)
model = WhisperModel("small", device="cpu", compute_type="int8")
print("Transcribing...", flush=True)
segments, info = model.transcribe(
    "source_16k.wav",
    language="ja",
    beam_size=5,
    word_timestamps=False,
    vad_filter=False,
)

results = []
for seg in segments:
    results.append({
        "start": seg.start,
        "end": seg.end,
        "text": seg.text.strip(),
    })
    print(f"[{seg.start:.2f} -> {seg.end:.2f}] {seg.text.strip()}", flush=True)

with open("faster_whisper_out.json", "w", encoding="utf-8") as f:
    json.dump(results, f, ensure_ascii=False, indent=2)

print(f"\nDone. {len(results)} segments saved to faster_whisper_out.json", flush=True)
