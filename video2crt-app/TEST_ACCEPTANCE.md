# Video2CRT 測試與驗收手冊

## 測試層級

| 層級 | 命令／方法 | 當輪狀態 |
| --- | --- | --- |
| 前端靜態建置 | `npm run build` | 已驗證，TypeScript/Vite 通過 |
| Rust 編譯 | `cargo check --manifest-path src-tauri/Cargo.toml` | 已驗證 |
| Rust 單元測試 | `cargo test --manifest-path src-tauri/Cargo.toml model_manager::tests -- --nocapture` | 已驗證，1 項通過 |
| Python 字幕契約 | `python scripts/test_subtitle_contract.py` 搭配 repo `src` | 已驗證，26 項通過 |
| 發布建置 | `powershell -File scripts/build-distributable.ps1` | 已驗證，NSIS 完成 |
| 安裝冒煙 | NSIS silent install + 啟動 5 秒 | 已驗證，exit 0、程式持續執行 |
| 高品質模型實際下載 | 應用程式內 3.2 GB 下載 | 未驗證，避免本輪下載大型模型 |
| 完整 GUI 到 `final.mp4` E2E | 真實影片、隔離輸出、視覺檢查 | 未驗證 |

## 發布包驗收

至少確認：

- 安裝程式可在一般 Windows 帳戶完成安裝。
- 安裝目錄包含 `video2crt.exe`、`runtime/tools/python/python.exe`、`ffmpeg.exe`、`yt-dlp.exe` 與標準模型。
- 啟動後能進入 URL 頁，不依賴開發機 checkout。
- 開始字幕工作時，缺少高品質模型會顯示確認視窗。
- 選擇標準模型能繼續工作；下載高品質模型時有進度與取消入口。

## 安全與資料

測試不可使用真實 API Key、Cookie、登入狀態或使用者影片。需要雲端翻譯時由測試者透過設定 UI 自行輸入，測試紀錄只保留「已設定／未設定」結果，不保留內容。

## 提交前檢查

```powershell
git diff --check
git diff --cached --check
rg -n -i "bearer|api[_ -]?key|password|secret|cookie|C:\\Users\\[^<]" <staged-files>
```

Pattern 命中後要人工判斷是文件中的去識別化說明還是真實憑證或私有路徑；後者必須從 staged set 移除。
