# Video2CRT Windows App 規格基準

## 目標

讓沒有開發環境的 Windows 使用者，從 YouTube URL 完成 CRT 影片與字幕輸出。安裝後的基本功能必須不依賴系統 Python、ffmpeg、Node.js 或 yt-dlp。

## 範圍

- Tauri 2 + React/Vite 桌面 UI。
- yt-dlp 下載公開影片，保留明確錯誤條件下的有限相容回退。
- ffmpeg cropdetect、libplacebo CRT shader、字幕燒錄與原始音訊封裝。
- 本機 faster-whisper ASR、本機 M2M100 翻譯與可選 MiniMax 雲端翻譯。
- 內建標準模型與使用者確認後下載的高品質模型。
- 使用者層級 NSIS 安裝，不要求系統管理員權限。

## 不在範圍

- 不自動取得 YouTube Cookie、登入狀態或 PO Token。
- 不把使用者影片或本機 ASR 音訊上傳到雲端。
- 不在主安裝包內合併超過 NSIS 單檔限制的大型模型。
- 不覆寫既有輸出資料夾或既有影片成果。

## 使用者流程

`URL → 選項 → 模型檢查 → 下載 → cropdetect → CRT render → ASR/翻譯 → burn → mux → final.mp4`

字幕模式：

| 模式 | 輸出 | 模型需求 |
| --- | --- | --- |
| 原文 | 一行原文字幕 | ASR 或可驗證的同語言 YouTube 字幕 |
| 原文＋繁中 | 非中文兩行；中文繁中單行 | ASR + 本機或雲端翻譯 |
| 無字幕 | 不產生／不燒錄字幕 | 不需要 ASR／翻譯模型 |

## 模型決策

- 標準模型隨主安裝包提供，確保可離線啟動。
- 高品質模型不靜默下載。字幕流程開始前若缺少模型，UI 顯示大小、網路需求與「安裝／使用標準／稍後」選項。
- 高品質模型下載至 `%LOCALAPPDATA%\Video2CRT\models\large`，每個檔案先寫入 `.part`，全部完成後才寫入 manifest 並切換使用路徑。
- 取消或失敗時維持標準模型，不能把半成品當作已安裝模型。

## 可測驗收條件

1. 全新 Windows 使用者可安裝並啟動主程式，不需另裝 Python、ffmpeg、Node.js、yt-dlp。
2. 沒有字幕需求時不顯示高品質模型提示。
3. 有字幕需求且高品質模型缺少時顯示確認畫面；拒絕後仍可使用標準模型。
4. 高品質模型下載可顯示進度並取消；未完成下載不會切換模型。
5. `final.mp4` 含視訊與原始音訊；字幕 SRT 時間不倒退、不重疊。
6. API Key 只進 Windows Credential Manager，不進檔案、日誌或 Git。
7. 發布包可由 `scripts/build-distributable.ps1` 建置，並產生 SHA-256。

## 已定案決策

- 主包使用 CPU 可工作的標準模型，避免超過 NSIS 2 GB 映射限制。
- 高品質模型採應用程式內明確確認後下載，不使用安裝程式偷偷連線下載。
- 發布包採 current-user 安裝與離線 WebView2 installer。
