# Video2CRT Architecture

How the project is organized.

```
Video2CRT/                     ← repo root
├── README.md                  ← 入口
├── HANDOFF.md                 ← 給 agent / 未來的自己
├── CHANGELOG.md               ← 歷史
├── LICENSE                    ← MIT
├── VERSION                    ← 0.5.0
├── .gitignore
├── .editorconfig
├── Makefile                   ← `make install`, `make test`, etc.
│
├── docs/                      ← 所有詳細文件
│   ├── architecture.md        ← (this file)
│   ├── workflow.md            ← 11 階段完整流程
│   ├── recipes.md             ← 場景食譜
│   ├── troubleshooting.md     ← 常見症狀與解決
│   └── index.md               ← 11 支影片目錄
│
├── scripts/                   ← CLI tools (給人或 agent 執行)
│   ├── install_skill.py       ← 驗證 skill + 環境
│   ├── run_whisper.py         ← 跑 Whisper 3-stage
│   └── crt.glsl               ← 共用 GLSL shader
│
├── src/                       ← 程式碼 (importable)
│   └── video2crt/             ← Python package
│       ├── __init__.py
│       ├── subtitle.py        ← SRT builder
│       ├── asr.py             ← Whisper integration
│       └── pipeline.py        ← ffmpeg pipeline
│
├── tests/                     ← 單元測試
│   ├── test_install_skill.py
│   ├── test_whisper_stages.py
│   └── run_all.py             ← 入口: `python tests/run_all.py`
│
├── output/                    ← 影片輸出 (8 支完整, 在子資料匣)
│   └── yt_<video-id>/
│       ├── source.mp4
│       ├── final.mp4
│       ├── raw.mp4
│       ├── crt.glsl
│       ├── faster_whisper_out.json
│       ├── zh-Hant.srt
│       └── handoff.md
│
└── archive/                   ← 不完整或失敗的影片 (3 支)
    ├── yt_CaCSuzR4DwM/        ← partial, raw only
    ├── yt_southern_cross/     ← raw only
    └── yt_videoB/             ← missing source.mp4
```

## 文件/程式碼分離原則

- **`docs/`** 是給人讀的文件（Markdown）
- **`src/video2crt/`** 是可 import 的 Python 模組
- **`scripts/`** 是給人或 agent 執行的 CLI 工具
- **`tests/`** 是驗證程式碼正確性的單元測試
- **`output/`** 是資料輸出（影片檔、字幕）
- **`archive/`** 是失敗或不完整的副產品

## 流程：從 YouTube URL 到 final.mp4

```
YouTube URL
  ↓  scripts/run_whisper.py          (Stage 4: Whisper 3-stage)
  ↓  src/video2crt/subtitle.py      (Stage 7: build SRT)
  ↓  src/video2crt/pipeline.py      (Stage 6+8+9: ffmpeg)
  ↓
output/yt_<id>/final.mp4
```

## Skill 與專案的關係

- **Skill** lives at `%LOCALAPPDATA%/hermes/skills/video-crt-geom-libplacebo/`
- **GitHub mirror**: `wuzkkavin/HermesFullSetup`
- Latest commit: `999086f` (gotchas 21+22 pre-flight + output dir rule)
- Contains 22 numbered items: pre-flight (gotcha 0) + 1-20 main + 21 + 22
- Project-level skill via Python package (`src/video2crt/`) is mostly procedural glue

## 擴充方向

- 新增一支影片：丟 URL → 跑 scripts/ + src/ → 輸出到 output/
- 新 gotcha 學到：更新 skill → commit → push to GitHub
- 新工具：加 scripts/, 對應 src/ 模組 + tests/
- 新環境變數：更新 install_skill.py candidates 列表

最後更新：2026-09-04
