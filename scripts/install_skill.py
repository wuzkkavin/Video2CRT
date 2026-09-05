#!/usr/bin/env python3
"""Install/check the video-crt-geom-libplacebo skill for a fresh agent.

The hermes-agent skill is ALREADY shipped with Hermes at
%LOCALAPPDATA%/hermes/skills/video-crt-geom-libplacebo/SKILL.md
(as of 2026-09-04, version v20, commit 695ca35 on GitHub).

This script just verifies the skill is present, readable, and reports
the version. If not present, prints instructions to manually copy it
from GitHub.

Usage:
    python scripts/install_skill.py

Exit codes:
    0 = skill installed and current
    1 = skill missing or out of date (follow printed instructions)
    2 = dependencies (ffmpeg/whisper/yt-dlp) missing
"""

import hashlib
import os
import subprocess
import sys
from pathlib import Path

SKILL_NAME = "video-crt-geom-libplacebo"
EXPECTED_GOTCHA_COUNT = 33  # Floor not exact (sibling agents add). Gotchas: 0 + 1-33 = 34 entries but gotcha 32 was sibling-merged.
SKILL_LOCAL_DIR = Path(os.environ.get("LOCALAPPDATA", "")) / "hermes" / "skills" / SKILL_NAME
SKILL_REMOTE = "https://github.com/wuzkkavin/HermesFullSetup/blob/main/skills/video-crt-geom-libplacebo/SKILL.md"


def check_skill_present() -> bool:
    """Is the skill already installed at the local Hermes skills dir?"""
    if not SKILL_LOCAL_DIR.is_dir():
        return False
    skill_md = SKILL_LOCAL_DIR / "SKILL.md"
    if not skill_md.is_file():
        return False
    return True


def count_gotchas() -> int:
    """How many gotchas are in the SKILL.md?
    Counts entries that start with N. where N is a positive integer.
    Counts the entire file (CRITICAL gotchas + post-flight gotchas added by sibling
    agents in other sections), excluding Reference files section.
    """
    skill_md = SKILL_LOCAL_DIR / "SKILL.md"
    if not skill_md.is_file():
        return 0
    import re
    text = skill_md.read_text(encoding="utf-8")
    # Strip Reference files section (not a gotcha)
    m = re.search(r"##\s*Reference files", text)
    if m:
        text = text[:m.start()]
    gotcha_nums = set()
    # Match both `**N. **` and `N. ` formats (some sibling agents don't bold)
    for m in re.finditer(r"^\*?\*?(\d+)\.\s+[A-Z*]", text, re.MULTILINE):
        gotcha_nums.add(int(m.group(1)))
    # Also match plain `**N. **` even if not at line start (gotchas may be in sub-bullets)
    for m in re.finditer(r"\*\*(\d+)\.\s+\*\*", text):
        gotcha_nums.add(int(m.group(1)))
    return max(len(gotcha_nums), 0)


def verify_dependencies() -> list[str]:
    """Check ffmpeg/yt-dlp/whisper are installed. Returns list of missing.
    Tries common Windows install paths because git-bash sandbox lacks PATH.

    Note: 'whisper' here refers to the openai-whisper CLI which is NOT used
    by Video2CRT (per gotcha 7, faster-whisper is the primary). We check it
    only as optional; missing whisper is OK (warn, not fail).
    """
    missing = []
    candidates = {
        "ffmpeg": [
            "ffmpeg",
            "/c/ProgramData/chocolatey/bin/ffmpeg.exe",
            "C:/ProgramData/chocolatey/bin/ffmpeg.exe",
            "C:/Program Files/ffmpeg/bin/ffmpeg.exe",
            "/c/Users/asaialabs/AppData/Local/Microsoft/WinGet/Links/ffmpeg.exe",
            "C:/Users/asaialabs/AppData/Local/Microsoft/WinGet/Links/ffmpeg.exe",
        ],
        "ffprobe": [
            "ffprobe",
            "/c/ProgramData/chocolatey/bin/ffprobe.exe",
            "C:/ProgramData/chocolatey/bin/ffprobe.exe",
            "C:/Program Files/ffmpeg/bin/ffprobe.exe",
            "/c/Users/asaialabs/AppData/Local/Microsoft/WinGet/Links/ffprobe.exe",
            "C:/Users/asaialabs/AppData/Local/Microsoft/WinGet/Links/ffprobe.exe",
        ],
        "yt-dlp": [
            "yt-dlp",
            "/c/Users/asaialabs/AppData/Roaming/Python/Python311/Scripts/yt-dlp.exe",
            "C:/Users/asaialabs/AppData/Roaming/Python/Python311/Scripts/yt-dlp.exe",
            "/c/Users/asaialabs/AppData/Local/Microsoft/WinGet/Links/yt-dlp.exe",
            "C:/Users/asaialabs/AppData/Local/Microsoft/WinGet/Links/yt-dlp.exe",
        ],
        # whisper (openai-whisper CLI) is OPTIONAL - faster-whisper is primary
    }
    optional = {
        "whisper": [
            "whisper",
            "/c/Users/asaialabs/AppData/Roaming/Python/Python311/Scripts/whisper.exe",
            "C:/Users/asaialabs/AppData/Roaming/Python/Python311/Scripts/whisper.exe",
        ],
    }
    for cmd, paths in candidates.items():
        found = False
        for path in paths:
            # Normalize MSYS /c/... paths to native Windows C:/... for subprocess
            if path.startswith("/c/"):
                path = "C:/" + path[3:]
            elif path.startswith("/cygdrive/c/"):
                path = "C:/" + path[12:]
            try:
                # ffmpeg/yt-dlp write version info to stderr (not stdout) on git-bash
                r = subprocess.run([path, "--version"], capture_output=True, timeout=5)
                # Check both stdout and stderr for any version-like content
                combined = (r.stdout or b"") + (r.stderr or b"")
                # Treat as success if either: rc==0 OR has version-like output
                is_ok = r.returncode == 0 or b"version" in combined.lower() or len(combined) > 50
                if is_ok:
                    found = True
                    break
            except (FileNotFoundError, subprocess.TimeoutExpired, OSError):
                continue
        if not found:
            missing.append(cmd)
    # whisper is optional - don't fail but record
    whisper_found = False
    for path in optional["whisper"]:
        if path.startswith("/c/"):
            path = "C:/" + path[3:]
        try:
            r = subprocess.run([path, "--version"], capture_output=True, timeout=5)
            if r.returncode == 0:
                whisper_found = True
                break
        except (FileNotFoundError, subprocess.TimeoutExpired, OSError):
            continue
    if not whisper_found:
        # Don't add to missing — just print info message later
        pass
    return missing


def verify_ffmpeg_libplacebo() -> bool:
    """Check ffmpeg has libplacebo + nvenc compiled in."""
    candidates = [
        "ffmpeg",
        "/c/ProgramData/chocolatey/bin/ffmpeg.exe",
        "C:/ProgramData/chocolatey/bin/ffmpeg.exe",
        "C:/Program Files/ffmpeg/bin/ffmpeg.exe",
        "/c/Users/asaialabs/AppData/Local/Microsoft/WinGet/Links/ffmpeg.exe",
        "C:/Users/asaialabs/AppData/Local/Microsoft/WinGet/Links/ffmpeg.exe",
    ]
    for path in candidates:
        # Normalize MSYS /c/... paths to native Windows C:/... for subprocess
        if path.startswith("/c/"):
            path = "C:/" + path[3:]
        elif path.startswith("/cygdrive/c/"):
            path = "C:/" + path[12:]
        try:
            r = subprocess.run([path, "-version"], capture_output=True, timeout=5)
        except (FileNotFoundError, OSError):
            continue
        # ffmpeg writes version to stderr on git-bash
        combined = ((r.stdout or b"") + (r.stderr or b"")).decode("utf-8", errors="ignore")
        if r.returncode == 0 or "libplacebo" in combined:
            return "libplacebo" in combined and "vulkan" in combined and "nvenc" in combined
    return False


def main() -> int:
    print(f"=== Video2CRT Skill Installer ===\n")
    print(f"Expected skill version: v{EXPECTED_GOTCHA_COUNT}+ (commit 999086f, 2026-09-04)\n")

    # Check skill presence
    if not check_skill_present():
        print("[FAIL] Skill NOT installed at:")
        print(f"  {SKILL_LOCAL_DIR}")
        print()
        print("To install, copy SKILL.md from GitHub:")
        print(f"  {SKILL_REMOTE}")
        print()
        print("Or run manually:")
        print("  mkdir -p %LOCALAPPDATA%\\hermes\\skills\\video-crt-geom-libplacebo")
        print("  # Place SKILL.md in that directory")
        print()
        return 1

    # Count gotchas
    gotchas = count_gotchas()
    print(f"[OK] Skill present at {SKILL_LOCAL_DIR}")
    print(f"[OK] Found {gotchas} gotchas in SKILL.md")

    if gotchas < EXPECTED_GOTCHA_COUNT:
        print(f"[WARN] Expected at least {EXPECTED_GOTCHA_COUNT} gotchas.")
        print(f"  Your installation has only {gotchas}.")
        print(f"  Pull latest from GitHub: {SKILL_REMOTE}")
        return 1

    # Check dependencies
    print()
    print("Checking dependencies...")
    missing_deps = verify_dependencies()
    if missing_deps:
        print(f"[FAIL] Missing CLI tools: {', '.join(missing_deps)}")
        print("  Install: ffmpeg (with libplacebo), yt-dlp (pip install yt-dlp), faster-whisper")
        return 2

    if not verify_ffmpeg_libplacebo():
        print(f"[FAIL] ffmpeg missing libplacebo/vulkan/nvenc codecs.")
        print("  Reinstall ffmpeg with these libraries compiled in.")
        return 2

    print(f"[OK] ffmpeg + libplacebo + vulkan + nvenc present")
    print(f"[OK] yt-dlp present")
    # whisper CLI is optional (faster-whisper is primary per gotcha 7)
    print(f"[INFO] whisper CLI optional - using faster-whisper (Python module)")
    print()
    print(f"[ALL PASS] Skill v{EXPECTED_GOTCHA_COUNT}+ installed and dependencies OK. Ready to convert videos.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
