from faster_whisper import WhisperModel
import json

model = WhisperModel("medium", device="cpu", compute_type="int8")
segments, info = model.transcribe(
    "source_16k.wav",
    beam_size=10,
    word_timestamps=True,
    vad_filter=False,
    condition_on_previous_text=False,
)
print(f"detected lang={info.language} prob={info.language_probability:.2f}")

results = []
for s in segments:
    words = getattr(s, "words", None) or []
    avg_logprob = sum(w.probability for w in words) / max(len(words), 1) if words else 0.5
    results.append({
        "start": round(s.start, 2),
        "end": round(s.end, 2),
        "text": s.text.strip(),
        "avg_logprob": round(avg_logprob, 3),
    })

with open("faster_whisper_out.json", "w", encoding="utf-8") as f:
    json.dump(results, f, ensure_ascii=False, indent=2)
print(f"Stage 1 (full multi, autodetect): {len(results)} segments")
for r in results[:30]:
    print(f"  {r['start']:6.2f}-{r['end']:6.2f} p={r['avg_logprob']:.2f} {r['text']}")
