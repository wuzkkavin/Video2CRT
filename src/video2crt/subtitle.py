"""Subtitle builder - 4-condition filter, gap control, and SRT emission.

Implements gotchas 9, 13, 15, 17 from video-crt-geom-libplacebo skill.
"""
from typing import Iterable


def is_skip(text: str) -> bool:
    """Gotcha 13: 4-condition skip filter.

    Skip ONLY when:
      1. text is silence marker ("-" or empty)
      2. text matches initial_prompt verbatim OR contains prompt keywords > 5 times (gotcha 14)
      3. char-spam: text > 10 chars but len(set(text)) < 5
      4. ?? fourth condition is YouTube watermark (handled separately by is_yt_watermark)
    """
    if not text or text == "-":
        return True
    for kw in ["日本語", "ロックライブ", "歌詞"]:
        if text == kw or text.count(kw) > 5:
            return True
    if len(set(text)) < 5 and len(text) > 10:
        return True
    return False


def is_yt_watermark(text: str) -> bool:
    """Gotcha 15: filter YouTube end-screen watermark phrases."""
    text_lower = text.lower()
    keywords = [
        "チャンネル登録", "subscribe", "登録して", "サブタイトル",
        "字幕 請訂閱", "高評価", "いいね",
    ]
    return any(kw in text_lower for kw in keywords)


def fmt_time(t: float) -> str:
    """Convert seconds to SRT timestamp HH:MM:SS,mmm"""
    h = int(t // 3600)
    m = int((t % 3600) // 60)
    s = t - h * 3600 - m * 60
    return f"{h:02d}:{m:02d}:{s:06.3f}".replace(".", ",")


def build_srt(
    segments: Iterable[dict],
    translations: dict[str, str],
    end_margin_s: float = 0.5,
    gap_s: float = 0.3,
    min_duration_s: float = 1.5,
    min_sep_s: float = 0.5,
) -> str:
    """Build SRT with bilingual two-line format.

    Args:
        segments: iterable of {start, end, text} dicts (from Whisper)
        translations: {asr_text -> trad_chinese_text} dict
        end_margin_s: trim segment end by this much (default 0.5s)
        gap_s: minimum gap between segments (default 0.3s, gotcha 9)
        min_duration_s: minimum subtitle duration (default 1.5s, expand short segments)
        min_sep_s: minimum separator gap between subtitle segments
    Returns:
        SRT-formatted string with two lines (original + Chinese) per entry
    """
    srt = []
    prev_end = 0
    i = 0
    for s in segments:
        text = s["text"]
        if is_skip(text):
            continue
        if is_yt_watermark(text):
            continue
        zh = translations.get(text)
        if not zh:
            continue
        i += 1
        ns = max(s["start"] + gap_s, prev_end + min_sep_s)
        ne = max(s["end"] - end_margin_s, ns + min_duration_s)
        prev_end = ne
        srt.append(f"{i}")
        srt.append(f"{fmt_time(ns)} --> {fmt_time(ne)}")
        srt.append(text)
        srt.append(zh)
        srt.append("")
    return "\n".join(srt)
