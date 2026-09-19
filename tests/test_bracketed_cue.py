"""Bracketed sound-cue filter (non-speech markers only).

Run via: $env:PYTHONPATH=(Resolve-Path 'src').Path; python tests/run_all.py
"""
import unittest

from video2crt.subtitle import is_bracketed_cue


class TestBracketedCue(unittest.TestCase):
    def test_bracketed_cues_dropped(self):
        for cue in ('["Pomp and Circumstance"]', "[Music]", "[Applause]",
                    "(applause)", "(Laughter)", "[音乐]", "  [Music]  ",
                    "()", "[]"):
            with self.subTest(cue=cue):
                self.assertTrue(is_bracketed_cue(cue), f"{cue!r} should be filtered")

    def test_real_speech_kept(self):
        for speech in ("Bullshit",  # gotcha 17, kept
                       "Kick the hop the shit you",  # gotcha 17, kept
                       "Hello (world)",  # brackets inside, not wrapped
                       "[ starts but never ends",
                       "ends but never starts ]",
                       "",
                       "LA LA LA LOVE SONG"):
            with self.subTest(speech=speech):
                self.assertFalse(is_bracketed_cue(speech), f"{speech!r} must be kept")


if __name__ == "__main__":
    unittest.main(verbosity=2)
