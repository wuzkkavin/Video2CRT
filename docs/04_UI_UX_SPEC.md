# Video2CRT UI／UX 規格

- Audience: UI/UX、前端 RD、測試與產品
- Authority: current
- Status: 結構已驗證；完整乾淨環境視覺 E2E 未驗證
- Last verified: 2026-09-18
- Evidence: `src/pages/UrlPage.tsx`、`OptionsPage.tsx`、`ProgressPage.tsx`、`DonePage.tsx`；模型選擇屬於 NSIS 安裝流程，不屬於運行 UI。

## 導覽與畫面

1. **URL Page**：輸入單一公開 YouTube URL，按「下一步」。空值或格式錯誤不得進入選項頁。
2. **Options Page**：裁切 `W:H:X:Y`、ASR 語言、字幕輸出、翻譯方式、模型與輸出資料夾；「開始轉檔」啟動後鎖定。
3. **Installer model choice**：NSIS 安裝程序顯示估計容量、網路需求與下載／略過選項；主程式運行期間不得顯示模型選擇。
4. **Progress Page**：顯示下載、裁切偵測、CRT、ASR、翻譯、字幕燒錄、封裝 stage、百分比、耗時、取消與錯誤。
5. **Done Page**：顯示 `final.mp4`、`zh-Hant.srt` 與輸出資料夾開啟入口；可重新開始。
6. **Settings Dialog**：輸入／刪除 MiniMax Key，僅顯示已設定狀態，不回填 key。

## 互動狀態

所有頁面需有正常、載入、空值、錯誤、取消與完成狀態。工作期間禁止重複提交；安裝器下載中的失敗不可啟用半成品。錯誤訊息需指出使用者可處理的下一步，不能要求使用者安裝系統依賴。

## 內容與視覺契約

- 繁中介面使用明確動詞：下一步、開始轉檔；模型下載／略過文字只出現在安裝器。
- 字幕模式需明示中文原文是一行、非中文雙語是兩行、無字幕略過 ASR／翻譯。
- 進度訊息不能暴露 API Key、Cookie、原始 access token 或完整私密路徑。
- 完成頁只在 `final.mp4` 確實存在且 pipeline 回報成功後顯示成功。

## UI 驗收

已完成 TypeScript/Vite production build 與元件結構檢查；完整 GUI 操作、鍵盤／輔助技術、不同 DPI 與乾淨 Windows 視覺驗收仍列為未驗證，不能以 build 通過取代。
