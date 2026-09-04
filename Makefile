# Video2CRT Makefile
# Run with: make <target>
# Windows: Get make.exe via choco or use mingw; git-bash users can use the targets directly

.PHONY: all install check test clean pipeline verify lint help

# Default: print available targets
help:
	@echo "Video2CRT Makefile"
	@echo ""
	@echo "Targets:"
	@echo "  install    Install + verify skill + dependencies"
	@echo "  check      Run install_skill.py (verify environment)"
	@echo "  test       Run pytest in tests/"
	@echo "  clean      Remove temp files (*.wav, *.jpg, clip_*)"
	@echo "  pipeline   Run full conversion pipeline (WIP)"
	@echo "  verify     Visual verification of final.mp4 (WIP)"
	@echo "  lint       Lint Python files"
	@echo ""

# Install + verify skill
install:
	pip install faster-whisper yt-dlp

# Run install_skill.py (verify skill + dependencies)
check:
	cd /c/Users/asaialabs/Documents/Hermes/Video2CRT && python scripts/install_skill.py

# Run tests
test:
	cd /c/Users/asaialabs/Documents/Hermes/Video2CRT && python -m pytest tests/ -v

# Clean temp files inside output/* (keep final.mp4 and SRTs)
clean:
	find /c/Users/asaialabs/Documents/Hermes/Video2CRT/output -name "clip_*.wav" -delete
	find /c/Users/asaialabs/Documents/Hermes/Video2CRT/output -name "*.jpg" -delete
	find /c/Users/asaialabs/Documents/Hermes/Video2CRT/output -name "source_16k.wav" -delete
	@echo "Cleaned temp files (.wav, .jpg)"

# Run full pipeline for a video (WIP - see pipeline.py stub)
pipeline:
	cd /c/Users/asaialabs/Documents/Hermes/Video2CRT && python -m video2crt.pipeline $(VIDEO_ID)

# Visual verification of a final.mp4
verify:
	cd /c/Users/asaialabs/Documents/Hermes/Video2CRT && python -m video2crt.verify $(VIDEO_ID)

# Lint
lint:
	cd /c/Users/asaialabs/Documents/Hermes/Video2CRT && python -m flake8 scripts/ src/ || echo "No flake8 installed; skipping"
