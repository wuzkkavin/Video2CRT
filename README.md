# Video2CRT

[![Version](https://img.shields.io/badge/version-0.5.0-blue.svg)](VERSION)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Python 3.11+](https://img.shields.io/badge/python-3.11+-blue.svg)](https://www.python.org)
[![ffmpeg libplacebo](https://img.shields.io/badge/ffmpeg-libplacebo-green.svg)](https://libplacebo.org)

把 YouTube 影片**轉成傳統 CRT 電視畫面** + **燒上雙語字幕**（原文 + 繁體中文）的工具專案。

## 用途

丟一個 YouTube 連結 → 自動產生一隻帶 CRT 效果（RGB phosphor dotmask + scanlines）跟中英/中日對照字幕的 1920×1080 MP4。

```
YouTube URL
   ↓ yt-dlp
   ↓ faster-whisper ASR (medium model) → SRT
   ↓ ffmpeg libplacebo shader (//!HOOK MAIN) → raw.mp4
   ↓ ffmpeg subtitles filter → subtitled.mp4
   ↓ ffmpeg -aspect 16:9 mux → final.mp4
```

## 專案結構

```
Video2CRT/
├── README.md         ← 你在這
├── HANDOFF.md        ← 給未來對話/agent 接手的指引
├── CHANGELOG.md
├── VERSION (0.5.0)
├── LICENSE (MIT)
├── Makefile          ← `make test` 等指令
│
├── docs/             ← 詳細文件
│   ├── architecture.md     ← 專案結構圖
│   ├── workflow.md         ← 11 階段完整流程
│   ├── recipes.md          ← 場景食譜（英文歌/日文 live/電影片段）
│   ├── troubleshooting.md  ← 常見問題排除
│   └── index.md            ← 11 支影片目錄
│
├── scripts/          ← CLI 工具
│   ├── install_skill.py
│   ├── run_whisper.py
│   └── crt.glsl
│
├── src/              ← Python 程式碼
│   └── video2crt/
│       ├── subtitle.py
│       ├── asr.py
│       └── pipeline.py
│
├── tests/            ← 單元測試
│   ├── test_install_skill.py
│   ├── test_whisper_stages.py
│   └── run_all.py   ← `python tests/run_all.py`
│
├── output/           ← 影片產出（8 支完整）
│   └── yt_<video-id>/
│
└── archive/          ← 不完整的（3 支）
```

## 開始使用

### 1. 驗證環境

```bash
cd "C:/Users/asaialabs/Documents/Hermes/Video2CRT"
python scripts/install_skill.py
```

必須看到：
```
[ALL PASS] Skill v20 installed and dependencies OK. Ready to convert videos.
```

### 2. 跑測試

```bash
python tests/run_all.py
```

13 tests, 全部應該 OK。

### 3. 新增一支影片

詳見 [`docs/recipes.md`](docs/recipes.md) 與 [`docs/workflow.md`](docs/workflow.md)。

## 核心 skill

`video-crt-geom-libplacebo` 在 Hermes 內部（`%LOCALAPPDATA%\hermes\skills\`），
GitHub mirror 在 `https://github.com/wuzkkavin/HermesFullSetup/blob/main/skills/video-crt-geom-libplacebo/`。

**20 個 gotcha**：完整學習教訓，從 11 支影片累積。

## 11 支影片摘要

8 支完成 (`output/`)，3 支 archive。詳見 [`docs/index.md`](docs/index.md)。

## 限制

- **ASR**: faster-whisper medium (CPU int8, ~3-5 min/3-min video)
- **GPU**: NVIDIA + CUDA + libplacebo
- **輸出**: 必須 1920×1080

最後更新：2026-09-04
