"""Caption policy: Chinese once; other languages original + zh-Hant.

Local inference uses already-installed CTranslate2 and SentencePiece. Only
pinned model DATA are fetched; no remote Python, credentials or subtitle upload.
"""
from __future__ import annotations

import json
import html
import math
import os
import re
from pathlib import Path

MODEL_REPO = "jncraton/m2m100_1.2B-ct2-int8"
MODEL_REVISION = "e50078df6be13a88592b70ea42f4d74c2082e448"
MODEL_FILES = ["config.json", "model.bin", "shared_vocabulary.json", "sentencepiece.bpe.model"]


_CAPTION_TIME = re.compile(
    r"(?P<start>\d{1,2}:\d{2}:\d{2}[.,]\d{3})\s+-->\s+"
    r"(?P<end>\d{1,2}:\d{2}:\d{2}[.,]\d{3})"
)


def _caption_seconds(value: str) -> float:
    hours, minutes, seconds = value.replace(",", ".").split(":")
    return int(hours) * 3600 + int(minutes) * 60 + float(seconds)


def _clean_youtube_caption(lines: list[str]) -> str:
    text = " ".join(lines)
    # Automatic VTT can contain inline word timestamps and ordinary HTML tags.
    text = re.sub(r"<\d{1,2}:\d{2}:\d{2}[.,]\d{3}>", "", text)
    text = re.sub(r"<[^>]+>", "", text)
    text = html.unescape(text)
    text = " ".join(text.split()).strip(" ♪♫")
    if not text or re.fullmatch(r"[♪♫\s]+", text):
        return ""
    # Keep speech and lyrics, but omit pure accessibility sound labels.
    if re.fullmatch(r"\[[^\]]+\]|\([^\)]+\)", text):
        return ""
    return text


def load_youtube_captions(path: Path, language: str) -> list[dict]:
    """Parse an exact-language YouTube SRT/VTT track into caption segments."""
    raw = path.read_text(encoding="utf-8-sig", errors="replace")
    captions = []
    for block in re.split(r"\r?\n\s*\r?\n", raw):
        lines = [line.strip() for line in block.splitlines() if line.strip()]
        timing_index = next((i for i, line in enumerate(lines) if _CAPTION_TIME.search(line)), None)
        if timing_index is None:
            continue
        match = _CAPTION_TIME.search(lines[timing_index])
        text = _clean_youtube_caption(lines[timing_index + 1:])
        if not text:
            continue
        captions.append({
            "start": _caption_seconds(match.group("start")),
            "end": _caption_seconds(match.group("end")),
            "text": text,
            "language": language.lower().split("-")[0],
            "source": "youtube-caption",
        })
    return captions


def captions_match_language(captions: list[dict], source_language: str | None) -> bool:
    """Reject a translated YouTube track masquerading as the spoken original."""
    if not captions or not source_language:
        return False
    expected = source_language.lower().split("-")[0]
    if expected == "yue":
        expected = "zh"
    declared = {str(c.get("language", "")).lower().split("-")[0] for c in captions}
    if declared != {expected}:
        return False
    text = " ".join(str(c.get("text", "")) for c in captions)
    if expected == "ja":
        return bool(re.search(r"[぀-ヿ]", text))
    if expected == "ko":
        return bool(re.search(r"[가-힯]", text))
    if expected == "zh":
        return bool(re.search(r"[㐀-鿿]", text)) and not re.search(r"[぀-ヿ]", text)
    if expected == "en":
        return bool(re.search(r"\b[A-Za-z]{2,}\b", text)) and not re.search(
            r"[぀-ヿ㐀-鿿가-힯]", text)
    return True


def select_source_captions(
    asr_segments: list[dict], youtube_captions: list[dict], source_language: str | None
) -> list[dict]:
    """Use a complete exact-language YouTube track; otherwise retain ASR."""
    if len(youtube_captions) < 2 or not captions_match_language(youtube_captions, source_language):
        return asr_segments
    if asr_segments:
        asr_span = max(s["end"] for s in asr_segments) - min(s["start"] for s in asr_segments)
        caption_span = max(s["end"] for s in youtube_captions) - min(s["start"] for s in youtube_captions)
        # A short preview/teaser track must not erase speech later in the video.
        if asr_span > 8 and caption_span < asr_span * 0.55:
            return asr_segments
    return youtube_captions


def clip_captions_to_duration(captions: list[dict], duration: float) -> list[dict]:
    """Clip a full YouTube track to the actual local media duration."""
    clipped = []
    for caption in captions:
        start = max(0.0, float(caption["start"]))
        end = min(float(caption["end"]), duration)
        if start < duration and end > start:
            clipped.append({**caption, "start": start, "end": end})
    return clipped


def align_caption_translations(source: list[dict], translated: list[dict]) -> dict[str, str]:
    """Align an uploader-provided zh-Hant track to source cues by time."""
    assigned: dict[str, list[str]] = {}
    for target in translated:
        midpoint = (float(target["start"]) + float(target["end"])) / 2
        candidates = []
        for segment in source:
            overlap = max(0.0, min(target["end"], segment["end"]) - max(target["start"], segment["start"]))
            segment_midpoint = (float(segment["start"]) + float(segment["end"])) / 2
            if overlap > 0:
                candidates.append((abs(midpoint - segment_midpoint), segment))
        if not candidates:
            continue
        segment = min(candidates, key=lambda item: item[0])[1]
        parts = assigned.setdefault(segment["text"], [])
        text = traditional(str(target["text"]).strip())
        if text and text not in parts:
            parts.append(text)
    return {source_text: " ".join(parts) for source_text, parts in assigned.items() if parts}


def merge_parallel_caption_tracks(
    source: list[dict], translated: list[dict]
) -> tuple[list[dict], dict[str, str]]:
    """Merge differently segmented source/zh-Hant tracks into complete cue pairs."""
    source_count = len(source)
    parent = list(range(source_count + len(translated)))

    def find(node: int) -> int:
        while parent[node] != node:
            parent[node] = parent[parent[node]]
            node = parent[node]
        return node

    def union(left: int, right: int) -> None:
        left_root, right_root = find(left), find(right)
        if left_root != right_root:
            parent[right_root] = left_root

    for source_index, segment in enumerate(source):
        for translated_index, target in enumerate(translated):
            overlap = max(
                0.0,
                min(float(segment["end"]), float(target["end"]))
                - max(float(segment["start"]), float(target["start"])),
            )
            # Ignore tiny boundary drift so it cannot join neighbouring lyric lines.
            if overlap >= 0.5:
                union(source_index, source_count + translated_index)

    source_groups: dict[int, list[int]] = {}
    translated_groups: dict[int, list[int]] = {}
    for index in range(source_count):
        source_groups.setdefault(find(index), []).append(index)
    for index in range(len(translated)):
        translated_groups.setdefault(find(source_count + index), []).append(index)

    merged = []
    translations: dict[str, str] = {}
    for first_index in range(source_count):
        root = find(first_index)
        indices = source_groups[root]
        if first_index != indices[0]:
            continue
        parts = [str(source[index]["text"]).strip() for index in indices]
        text = " ".join(part for part in parts if part)
        segment = {
            **source[indices[0]],
            "start": min(float(source[index]["start"]) for index in indices),
            "end": max(float(source[index]["end"]) for index in indices),
            "text": text,
        }
        segment.pop("words", None)
        merged.append(segment)
        target_indices = translated_groups.get(root, [])
        if target_indices:
            zh_parts = [traditional(str(translated[index]["text"]).strip())
                        for index in target_indices]
            translations[text] = " ".join(part for part in zh_parts if part)
    return merged, translations


def traditional(text: str) -> str:
    """OpenCC phrase-aware conversion, bundled locally (no network)."""
    return _converter().convert(text)


from functools import lru_cache

@lru_cache(maxsize=1)
def _converter():
    import sys
    sys.path.insert(0, str(Path(__file__).parent / "vendor"))
    from opencc import OpenCC
    return OpenCC("s2t")


def caption_language(segment: dict) -> str:
    lang = (segment.get("language") or "").lower().split("-")[0]
    text = segment["text"]
    # Script checks catch short code-switches without classifying Japanese
    # kanji-only phrases as Chinese. The detected audio language wins for Han.
    if re.search(r"[\u3040-\u30ff]", text):
        return "ja"
    if re.search(r"[\uac00-\ud7af]", text):
        return "ko"
    if lang in ("zh", "yue"):
        if not re.search(r"[\u3400-\u9fff]", text) and re.search(r"[A-Za-z]{2}", text):
            return "en"
        return "zh"
    if not lang:
        raise ValueError("字幕缺少辨識語言，無法決定翻譯方向")
    return lang


def normalize_segments(segments: list[dict], duration: float | None = None) -> list[dict]:
    """Preserve spoken starts; clamp overlaps, never stretch a short cue.

    Invalid ASR timestamps fail explicitly instead of losing spoken text or
    emitting unplayable SRT. No arbitrary 0.5s gaps/delays or watermark keywords.
    """
    cleaned = []
    for segment in segments:
        text = " ".join(str(segment.get("text", "")).split())
        if not text or text == "-":
            continue
        start, end = float(segment["start"]), float(segment["end"])
        if not math.isfinite(start) or not math.isfinite(end) or end <= start:
            raise ValueError(f"字幕時間無效：{start} --> {end}；需要重新辨識此段")
        start = max(0.0, start)
        if duration:
            end = min(end, duration)
        if end <= start:
            raise ValueError("字幕超出影片長度，需要重新辨識此段")
        item = {**segment, "text": text, "start": start, "end": end}
        item["language"] = caption_language(item)
        cleaned.append(item)
    cleaned.sort(key=lambda s: (s["start"], -(s["end"] - s["start"])))
    # Word timestamp alignment sometimes emits two alternatives at exactly the
    # same millisecond. Keep the longer cue; a zero-length cue must never make
    # the entire conversion fail.
    deduplicated = []
    for item in cleaned:
        if deduplicated and round(item["start"] * 1000) == round(deduplicated[-1]["start"] * 1000):
            continue
        deduplicated.append(item)
    normalized = []
    for item in deduplicated:
        while normalized and item["start"] < normalized[-1]["end"]:
            normalized[-1]["end"] = min(normalized[-1]["end"], item["start"])
            if round(normalized[-1]["end"] * 1000) <= round(normalized[-1]["start"] * 1000):
                normalized.pop()
            else:
                break
        normalized.append(item)
    return normalized


def split_spoken_captions(segments: list[dict]) -> list[dict]:
    """Use word timing to keep one readable original line, with no made-up text."""
    result = []
    for segment in segments:
        words = segment.get("words") or []
        joined = "".join(w["word"] for w in words)
        if not words or re.sub(r"\s", "", joined) != re.sub(r"\s", "", segment["text"]):
            result.append(segment)
            continue
        group = []
        for i, word in enumerate(words):
            group.append(word)
            text = "".join(w["word"] for w in group).strip()
            width = sum(2 if ord(c) > 0x2fff else 1 for c in text)
            elapsed = word["end"] - group[0]["start"]
            last = i == len(words) - 1
            pause = not last and words[i + 1]["start"] - word["end"] > 0.45
            boundary = re.search(r"[。！？.!?，,、；;]$", text) and elapsed >= 1.2
            if last or (elapsed > 0.05 and (width >= 48 or elapsed >= 6.0 or pause or boundary)):
                if word["end"] <= group[0]["start"] and result:
                    result[-1]["text"] += text
                else:
                    result.append({**segment, "start": group[0]["start"], "end": word["end"],
                                   "text": text, "words": list(group)})
                group = []
    return result


def timestamp(seconds: float) -> str:
    ms = round(seconds * 1000)
    h, ms = divmod(ms, 3600000)
    m, ms = divmod(ms, 60000)
    s, ms = divmod(ms, 1000)
    return f"{h:02}:{m:02}:{s:02},{ms:03}"


def build_srt_two_line(segments, translations, *, emit_empty_translation=False, **_legacy):
    lines = []
    for i, seg in enumerate(normalize_segments(segments), 1):
        text = seg["text"]
        if seg["language"] == "zh":
            caption = traditional(text)
        else:
            zh = traditional(" ".join(translations.get(text, "").split()))
            if not zh and not emit_empty_translation:
                raise ValueError(f"缺少繁中翻譯 (translation)：第 {i} 段")
            # Escape ASS tags; SRT text must not execute formatting overrides.
            caption = text + ("\n" + zh if zh else "")
        caption = caption.replace("{", "｛").replace("}", "｝")
        caption = caption.replace("<", "＜").replace(">", "＞")
        lines.append(f"{i}\n{timestamp(seg['start'])} --> {timestamp(seg['end'])}\n{caption}\n")
    return "\n".join(lines)


def build_srt_original(segments):
    """Original-only subtitles. Chinese remains one Traditional Chinese line."""
    lines = []
    for i, seg in enumerate(normalize_segments(segments), 1):
        caption = traditional(seg["text"]) if seg["language"] == "zh" else seg["text"]
        caption = caption.replace("{", "｛").replace("}", "｝")
        caption = caption.replace("<", "＜").replace(">", "＞")
        lines.append(f"{i}\n{timestamp(seg['start'])} --> {timestamp(seg['end'])}\n{caption}\n")
    return "\n".join(lines)


class LocalTranslator:
    def __init__(self, progress=lambda message: None):
        import ctranslate2
        import sentencepiece
        from huggingface_hub import snapshot_download
        options = dict(repo_id=MODEL_REPO, revision=MODEL_REVISION,
                       allow_patterns=MODEL_FILES, token=False)
        bundled = os.environ.get("VIDEO2CRT_TRANSLATION_MODEL_DIR", "").strip()
        folder = bundled if bundled and all((Path(bundled) / name).is_file() for name in MODEL_FILES) else None
        try:
            if folder is None:
                folder = snapshot_download(**options, local_files_only=True)
            if not all((Path(folder) / name).is_file() for name in MODEL_FILES):
                raise FileNotFoundError("incomplete model cache")
        except Exception:
            # The exact revision and allowlist prohibit remote code downloads.
            progress("首次準備本機翻譯模型（約 1.3 GB），字幕留在本機")
            folder = snapshot_download(**options)
        self.model = ctranslate2.Translator(folder, device="cpu", compute_type="int8",
                                            inter_threads=1, intra_threads=1)
        self.sp = sentencepiece.SentencePieceProcessor(model_file=str(Path(folder) / "sentencepiece.bpe.model"))
        self.vocabulary = set(json.loads((Path(folder) / "shared_vocabulary.json").read_text(encoding="utf-8")))
        self.cache = {}

    def translate(self, text: str, language: str) -> str:
        key = (language, text)
        if key in self.cache:
            return self.cache[key]
        language = {"jw": "jv", "nn": "no"}.get(language, language)
        tag = f"__{language}__"
        if tag not in self.vocabulary:
            raise ValueError(f"本機翻譯暫不支援語言 {language}，可啟用 MiniMax 翻譯")
        tokens = [tag] + self.sp.encode(text, out_type=str) + ["</s>"]
        if len(tokens) > 500:
            raise ValueError("字幕段落過長，需要重新分段")
        zh = self._decode(tokens)
        if not self._valid(zh):
            # Beam search occasionally degenerates into repeated ⁇ on short
            # lyric lines. One greedy pass usually recovers a usable
            # translation; if that also fails, fail exactly as before.
            zh = self._decode(tokens, beam_size=1)
        if not self._valid(zh):
            raise ValueError("本機模型未產生有效中文翻譯，可改用 MiniMax 翻譯")
        self.cache[key] = zh
        return zh

    def _decode(self, tokens, beam_size=4) -> str:
        result = self.model.translate_batch([tokens], target_prefix=[["__zh__"]],
                                            beam_size=beam_size, max_decoding_length=512)[0]
        generated = [t for t in result.hypotheses[0] if not t.startswith("__") and t not in ("</s>", "<s>", "<pad>")]
        return traditional(self.sp.decode(generated).strip())

    @staticmethod
    def _valid(zh: str) -> bool:
        return bool(zh) and "<unk>" not in zh and bool(re.search(r"[\u3400-\u9fff]", zh))


def translate_locally(segments, progress=lambda message: None):
    pending = [s for s in segments if caption_language(s) != "zh"]
    if not pending:
        return {}
    translator = LocalTranslator(progress)
    translations = {}
    skipped = 0
    for i, s in enumerate(pending, 1):
        progress(f"本機翻譯 {i}/{len(pending)}")
        try:
            translations[s["text"]] = translator.translate(s["text"], caption_language(s))
        except ValueError as exc:
            # Untranslatable single lines (music-cue echoes, shouted
            # fragments the small model only repeats) must not kill the
            # whole video. The caller drops them from the SRT entirely:
            # neither translation nor original is shown for that cue.
            skipped += 1
            progress(f"略過無法翻譯的第 {i} 段：{exc}")
    if skipped:
        progress(f"共略過 {skipped} 段無法翻譯，保留其餘雙語字幕")
    return translations
