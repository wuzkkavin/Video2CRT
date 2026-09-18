# Video2CRT 測試與驗收手冊

## 本輪完整自動化驗證總覽

「26 項」只代表 APP 字幕契約測試，不代表整個專案的測試總數。本輪已執行並通過 **51 項自動化測試**：

- 根目錄 Python 回歸測試：13 項（`python tests/run_all.py`）。
- APP Python 字幕契約測試：26 項（需先設定 repo `src` 的 `PYTHONPATH`）。
- Rust `cargo test --all-targets`：12 項（11 項 library unit tests、1 項 orchestrator integration test；各 binary harness 僅編譯，沒有測試函式）。
- 合計：13 + 26 + 12 = 51 項通過。

測試前置檢查另由 `python scripts/install_skill.py` 執行；本機實測 **38 條 CRT skill gotcha**，最低新鮮度門檻為 36，並輸出 `[ALL PASS]`。gotcha 數量不是測試案例數量，兩者不可混用。

## 測試層級

| 層級 | 命令／方法 | 當輪狀態 |
| --- | --- | --- |
| 前端靜態建置 | `npm run build` | 已驗證，TypeScript/Vite 通過 |
| Rust 編譯 | `cargo check --manifest-path src-tauri/Cargo.toml` | 已驗證 |
| CRT skill 與依賴前置檢查 | `python scripts/install_skill.py` | 已驗證，38 條 gotcha，v36+ `[ALL PASS]` |
| 根目錄 Python 回歸 | `$env:PYTHONPATH=(Resolve-Path 'src').Path; python tests/run_all.py` | 已驗證，13 項通過 |
| Rust 全部測試目標 | `cargo test --manifest-path src-tauri/Cargo.toml --all-targets -- --test-threads=1` | 已驗證，12 項通過（11 library + 1 integration） |
| Python 字幕契約 | `$env:PYTHONPATH=(Resolve-Path '..\\src').Path; python scripts/test_subtitle_contract.py` | 已驗證，26 項通過 |
| 發布建置 | `powershell -File scripts/build-distributable.ps1` | 已驗證，NSIS 完成 |
| 安裝冒煙 | NSIS silent install + 啟動 5 秒 | 已驗證，exit 0、程式持續執行 |
| 高品質模型實際下載 | NSIS 安裝時 3.2 GB 下載 | 未驗證，避免本輪下載大型模型 |
| 完整 GUI 到 `final.mp4` E2E | 真實影片、隔離輸出、視覺檢查 | 未驗證 |

## 發布包驗收

至少確認：

- 安裝程式可在一般 Windows 帳戶完成安裝。
- 安裝目錄包含 `video2crt.exe`、`runtime/tools/python/python.exe`、`ffmpeg.exe`、`yt-dlp.exe` 與標準模型。
- 啟動後能進入 URL 頁，不依賴開發機 checkout。
- 安裝程序會顯示高品質模型下載／略過選項。
- 主程式開始字幕工作時不顯示模型選擇；安裝器下載失敗仍可使用標準模型。

## 安全與資料

測試不可使用真實 API Key、Cookie、登入狀態或使用者影片。需要雲端翻譯時由測試者透過設定 UI 自行輸入，測試紀錄只保留「已設定／未設定」結果，不保留內容。

## 提交前檢查

```powershell
git diff --check
git diff --cached --check
rg -n -i "bearer|api[_ -]?key|password|secret|cookie|C:\\Users\\[^<]" <staged-files>
```

Pattern 命中後要人工判斷是文件中的去識別化說明還是真實憑證或私有路徑；後者必須從 staged set 移除。
