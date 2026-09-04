"""Standalone test runner for install_skill verification.

Run via: python tests/test_install_skill.py
Or:     python tests/run_all.py
"""
import sys
import unittest
import os

# Add scripts/ to import path
SCRIPTS = os.path.join(os.path.dirname(os.path.dirname(__file__)), "scripts")
sys.path.insert(0, SCRIPTS)

from install_skill import (
    check_skill_present,
    count_gotchas,
    verify_dependencies,
    verify_ffmpeg_libplacebo,
    EXPECTED_GOTCHA_COUNT,
)


class TestSkillInstallation(unittest.TestCase):
    """Verify Hermes skill + CLI tools are ready for Video2CRT."""

    def test_skill_present(self):
        self.assertTrue(
            check_skill_present(),
            "Skill not found at %LOCALAPPDATA%/hermes/skills/",
        )

    def test_skill_version_at_least_20(self):
        n = count_gotchas()
        self.assertGreaterEqual(
            n, EXPECTED_GOTCHA_COUNT,
            f"Only {n} gotchas in SKILL.md, expected >= {EXPECTED_GOTCHA_COUNT}. "
            "Run: cd ~/AppData/Local/hermes && git pull",
        )

    def test_ffmpeg_present(self):
        self.assertNotIn("ffmpeg", verify_dependencies())

    def test_ffprobe_present(self):
        self.assertNotIn("ffprobe", verify_dependencies())

    def test_ytdlp_present(self):
        self.assertNotIn("yt-dlp", verify_dependencies())

    def test_ffmpeg_has_libplacebo_vulkan_nvenc(self):
        self.assertTrue(
            verify_ffmpeg_libplacebo(),
            "ffmpeg missing libplacebo/vulkan/nvenc codecs",
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
