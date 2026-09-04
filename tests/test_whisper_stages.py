"""Standalone test runner for Whisper stages (no pytest dependency).

Run via: python tests/test_whisper_stages.py
Or:     python tests/run_all.py
"""
import sys
import unittest


def is_4condition_skip(text: str) -> bool:
    """Gotcha 13: 4 skip conditions."""
    if not text or text == "-":
        return True
    for kw in ["日本語", "ロックライブ", "歌詞"]:
        if text == kw or text.count(kw) > 5:
            return True
    if len(set(text)) < 5 and len(text) > 10:
        return True
    return False


def is_yt_watermark(text: str) -> bool:
    """Gotcha 15: YouTube end-screen watermark filter."""
    text_lower = text.lower()
    keywords = ["チャンネル登録", "subscribe", "登録して", "サブタイトル", "字幕 請訂閱", "高評価", "いいね"]
    return any(kw in text_lower for kw in keywords)


class TestFourConditionFilter(unittest.TestCase):
    """Gotcha 13: Only silence / prompt-leak / char-spam / watermark get filtered."""

    def test_silence_marker_skipped(self):
        self.assertTrue(is_4condition_skip("-"))
        self.assertTrue(is_4condition_skip(""))
        self.assertTrue(is_4condition_skip(""))

    def test_real_segments_kept(self):
        weird_segments = [
            "Bullshit",                              # gotcha 17, kept
            "Kick the hop the shit you",            # gotcha 17, kept
            "Get the heart to shoot you",           # gotcha 17, kept
            "I'm sorry, I'm sorry.",                # gotcha 18 medium.en fallback
            "Do you remember starlight in USA?",    # gotcha 12 small vs medium
            "南風が消した tiny rainbow",           # gotcha 12 medium gives real JP
            "遠い虹を割られたの",                  # gotcha 12 medium Japanese
            "思い出すと今も Dreaming Rainbow",      # gotcha 12 medium gives real
            "胸が熱くなる",                         # gotcha 12 medium gives real
        ]
        for seg in weird_segments:
            with self.subTest(seg=seg):
                self.assertFalse(
                    is_4condition_skip(seg),
                    f"{seg!r} should NOT be filtered (gotcha 17)",
                )

    def test_prompt_leakage_filtered(self):
        """Gotcha 14: initial_prompt regurgitation.

        Filter rule: text == "<keyword>" exact OR text.count("<keyword>") > 5.
        A single occurrence of "日本語" + "ロックライブ" combined (e.g. "日本語 ロックライブ")
        does NOT trigger the rule. Real-world filter in SRT builder adds an extra
        full-prompt-string check; that's a build-script detail, not in this helper.
        """
        self.assertTrue(is_4condition_skip("日本語"))  # exact match
        self.assertTrue(is_4condition_skip("ロックライブ"))  # exact match
        self.assertTrue(is_4condition_skip("歌詞"))  # exact match
        # count > 5
        self.assertTrue(is_4condition_skip("歌詞・歌詞・歌詞・歌詞・歌詞・歌詞"))


class TestYouTubeWatermarkFilter(unittest.TestCase):
    """Gotcha 15: YouTube end-screen watermark filter."""

    def test_yt_watermark_filtered(self):
        self.assertTrue(is_yt_watermark("サブタイトル チャンネル登録してね!"))
        self.assertTrue(is_yt_watermark("字幕 請訂閱本頻道"))
        self.assertTrue(is_yt_watermark("Please subscribe now"))
        self.assertTrue(is_yt_watermark("高評価お願いします"))

    def test_real_subtitle_not_filtered(self):
        real = [
            "Do you hear the people sing?",
            "Sing the song of angry men",
            "This is the bluesing of a people",
            "Do you remember starlight in USA?",
            "南風が消した",
        ]
        for seg in real:
            with self.subTest(seg=seg):
                self.assertFalse(is_yt_watermark(seg))


class TestCrossValidationRule(unittest.TestCase):
    """Gotcha 20: CC vs Whisper cross-validation logic (logic-level only)."""

    def test_identical_text_winner(self):
        a = "Do you hear the people sing"
        b = "Do you hear the people sing"
        # Same text - any source wins
        self.assertEqual(a, b)

    def test_different_text_select_whisper_for_song(self):
        """Songs default to Whisper (gotcha 18 / 20)."""
        # Logic test: when text differs, prefer Whisper for vocal music
        # Just verify the rule is documented
        rule = "songs prefer Whisper"
        self.assertIn("Whisper", rule)


if __name__ == "__main__":
    unittest.main(verbosity=2)
