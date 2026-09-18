# Video2CRT 發布與版本說明

## 里程碑

- 應用程式版本：`0.1.0`
- 目標平台：Windows x64
- 安裝方式：Tauri 2 NSIS、current-user
- WebView2：offline installer
- 發布日期：2026-09-18

## 正式產物

| 產物 | 路徑 | 狀態 |
| --- | --- | --- |
| NSIS 安裝包 | `dist-distributable/Video2CRT_0.1.0_x64-setup.exe` | 已產生並完成安裝冒煙測試 |
| SHA-256 | `96AAEF70137210371A2E471727F7E4FDE9CECE3DA02B0047A5228265B8B66232` | 當輪已計算 |

主包包含 Python 3.12 embeddable runtime、faster-whisper/CTranslate2/SentencePiece/Hugging Face Hub、ffmpeg、Node.js、官方 yt-dlp、OpenCC、標準 ASR／翻譯模型與應用程式資源。高品質模型不放入主包，改由應用程式內確認後下載。

## 建置

在 Windows 建置機執行：

```powershell
npm install
npm run tauri:build:distributable
```

建置腳本會下載並 stage runtime，寫入 `runtime-manifest.json`，再呼叫 Tauri NSIS。`-SkipBuild` 只驗證 runtime staging；`-SkipModels` 只適合預演，不能產生可離線字幕發布包。

## 可重現性與界線

- 原始碼與建置腳本已進 Git，可重建發布包。
- `.packaging/`、`dist-distributable/`、Tauri target 與模型快取不進 Git。
- 下載資源包含固定版本或 revision；正式交付前應重新計算 hash。
- 本輪已驗證安裝程式 exit code 0，安裝後主程式成功啟動；尚未在乾淨 Windows 使用者帳戶執行完整影片 GUI E2E，也尚未實際下載完整 3.2 GB 高品質模型。

## 發布前核對

1. 執行 `python scripts/install_skill.py`，確認輸出 38 條 gotcha 且 `[ALL PASS]`；36 是最低門檻，不是總數。
2. 執行根目錄回歸測試：`$env:PYTHONPATH=(Resolve-Path 'src').Path; python tests/run_all.py`，確認 13 項通過。
3. 執行 `$env:PYTHONPATH=(Resolve-Path '..\\src').Path; python scripts/test_subtitle_contract.py`，確認 26 項通過。
4. 執行 `cargo check --manifest-path src-tauri/Cargo.toml`，再執行 `cargo test --manifest-path src-tauri/Cargo.toml --all-targets -- --test-threads=1`，確認 12 項通過。
5. 執行 `npm run build`、發布腳本，確認安裝包存在、大小合理、SHA-256 已記錄。
6. 在隔離資料夾 silent install，確認 `video2crt.exe`、embedded Python、ffmpeg、yt-dlp 與標準模型存在。
7. 不把 API Key、Cookie、影片、模型快取或本機日誌加入 commit。
