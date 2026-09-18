# Video2CRT Agent 接手指南

## 必讀順序

1. 先看根目錄 [HANDOFF.md](../HANDOFF.md) 最新日期段落。
2. 看本目錄 [SPEC.md](SPEC.md) 與 [DEVELOPMENT_GUIDE.md](DEVELOPMENT_GUIDE.md)。
3. 看 [RELEASE.md](RELEASE.md) 與 [TEST_ACCEPTANCE.md](TEST_ACCEPTANCE.md)。
4. 執行 `git status --short`，把父層影片輸出與私人工作檔視為既存資料，不要自動還原或刪除。

## 第一個安全動作

只讀檢查：

```powershell
git status --short
git log -5 --oneline --decorate
git remote -v
Get-Content video2crt-app/package.json
Get-Content video2crt-app/src-tauri/Cargo.toml
```

接著依改動範圍執行最小驗證；不要先執行會刪除 `.packaging/`、輸出資料或使用者資料的清理命令。

## Current truth

- Tauri 2 + React/Vite UI 入口是 `src/App.tsx`。
- Rust command 與 runtime 資源解析在 `src-tauri/src/lib.rs`、`orchestrator.rs`。
- 可選模型安裝在 `src-tauri/src/model_manager.rs`；前端視窗是 `src/components/ModelInstallDialog.tsx`。
- 正式 runtime 由 `scripts/build-distributable.ps1` 產生；模型 stage 設定在 `scripts/stage-models.py`。
- 標準模型內建；高品質模型只有在使用者明確確認後才下載。

## 變更矩陣

| 改動 | 最小驗證 |
| --- | --- |
| React/TS/CSS | `npm run build` |
| Rust command/orchestrator | `cargo check --manifest-path src-tauri/Cargo.toml` + 相關 `cargo test` |
| Python sidecar | 字幕契約測試，必要時隔離輸出 E2E |
| 打包腳本／資源 | `powershell -File scripts/build-distributable.ps1` + silent install |
| 模型管理 | status/manifest 單元測試；真實下載需另記錄網路未驗證狀態 |

## 不可越過的邊界

- 不提交 API Key、Cookie、模型快取、影片、原始影音或本機日誌。
- 不把使用者的私人路徑、OneDrive 內容或憑證寫入交接文件。
- 不覆寫或刪除父層 `output/`、`archive/` 或既有交付成果。
- 不把「編譯通過」寫成「完整 GUI E2E 通過」。
- 不把「安裝冒煙通過」寫成「高品質模型下載已通過」。

## 收工

更新根目錄 `HANDOFF.md`，記錄當輪 current truth、驗證層級、未完成與下一步。外部 commit/push 只在使用者明確授權時進行；stage 前逐檔查看 diff 與敏感資料掃描。
