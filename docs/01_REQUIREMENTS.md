# Video2CRT 需求與範圍基準

- Audience: 產品、SA、測試與接手開發者
- Authority: current
- Status: 已驗證（功能來自 `video2crt-app/SPEC.md` 與目前 UI／Rust／Python 實作）
- Last verified: 2026-09-18
- Evidence: `video2crt-app/SPEC.md`、`src/App.tsx`、`src-tauri/src/orchestrator.rs`、`TEST_ACCEPTANCE.md`

## 產品目標

讓沒有 Python、ffmpeg、Node.js 或 yt-dlp 開發環境的 Windows 使用者，貼上公開 YouTube URL 後產生 CRT 顯像 MP4、原文或雙語字幕與可追查的中間檔。

## 使用者與範圍

- 主要使用者：一般 Windows 使用者，能提供公開 YouTube URL。
- 維護者：可在 Windows 建置 Tauri、Rust、React/Vite 與 Python sidecar。
- 測試者：能依測試矩陣驗證字幕、影音封裝、安裝與發布產物。
- 接手 Agent：從 `HANDOFF.md` 與本索引取得 current truth。

包含：Tauri 2 UI、yt-dlp 下載、cropdetect、libplacebo CRT、faster-whisper ASR、本機／MiniMax 翻譯、字幕燒錄、原始音訊封裝、current-user NSIS 發布、高品質模型確認式下載。

不包含：Cookie／登入／PO Token 自動取得、使用者影片上傳雲端、主包內放入大型高品質模型、覆寫既有輸出、伺服器帳號與資料庫。

## 功能需求

| ID | 需求 | 驗收依據 |
| --- | --- | --- |
| REQ-001 | 安裝後不依賴系統 Python、ffmpeg、Node.js、yt-dlp 即可啟動 | 發布包檢查與 silent install |
| REQ-002 | 使用者可輸入 URL、設定裁切、ASR 語言、字幕模式、翻譯方式與輸出位置 | `UrlPage`、`OptionsPage` |
| REQ-003 | 建立以影片標題命名且不覆寫的輸出資料夾 | Rust orchestrator 測試 |
| REQ-004 | 下載、裁切、CRT、ASR、翻譯、燒錄、封裝有可見進度與錯誤 | `ProgressPage`、Tauri events |
| REQ-005 | 原文、原文＋繁中、無字幕三種模式行為固定 | 字幕契約 26 項 |
| REQ-006 | 安裝程序先取得高品質模型下載確認；略過後使用標準模型 | `installer-hooks.nsh`、`install-large-model.py`、模型管理測試 |
| REQ-007 | 翻譯未完成時不得燒錄不完整雙語成品 | 字幕契約與 pipeline guard |
| REQ-008 | API Key 只存 Windows Credential Manager，不落地到檔案或 log | `settings.rs` 與安全文件 |
| REQ-009 | `final.mp4` 保留視訊與原始音訊，字幕時間不倒退、不重疊 | ffprobe、字幕契約與 E2E 檢查 |
| REQ-010 | 建置腳本產生可驗證 SHA-256 的 NSIS 安裝包 | `build-distributable.ps1`、RELEASE |

## 非功能需求

- 隱私：ASR 與本機翻譯留在本機；只有使用者選擇雲端翻譯時才送出字幕文字。
- 可恢復性：模型使用 `.part` 與 manifest；取消或失敗不得啟用半成品。
- 相容性：Windows x64、Tauri 2、Rust stable、Node/npm 與內嵌 runtime。
- 可追查性：保留 source、raw、SRT、metadata 與可驗證的發布 hash。

## 驗收與變更控制

每項需求以 `REQ-*` 追蹤至設計、程式與 `TST-*`。未完成乾淨帳戶 GUI E2E 或高品質模型實際下載前，不得宣稱完整發布驗收通過。
