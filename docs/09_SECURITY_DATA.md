# Video2CRT 安全與資料處理規格

- Audience: 資安、RD、維運與發布審查
- Authority: current
- Status: 已驗證（程式與 staged 文件掃描）；外部服務政策仍可能變動
- Last verified: 2026-09-18
- Evidence: `settings.rs`、`TEST_ACCEPTANCE.md`、敏感字串掃描與 Git staged diff

## 資料分類與流向

| 分類 | 例子 | 流向與處理 |
| --- | --- | --- |
| Secret | MiniMax API Key、Cookie、token | 只進 Credential Manager／可信登入流程；不進 Git、log、Markdown |
| 使用者媒體 | source.mp4、WAV、字幕 | 預設留本機；不因 ASR 或 CRT 上傳 |
| 可外傳文字 | 外語字幕 | 只有使用者選雲端翻譯時送 MiniMax |
| 發布資源 | runtime、模型、hash | 來源與 revision 固定；不含使用者資料 |
| 診斷資料 | stage、非敏感錯誤 | 可保留供本機追查；先去除 key、Cookie、私密路徑 |

## 控制措施

- API Key UI 只顯示是否已設定，刪除透過 Credential Manager；不提供讀回內容。
- 模型以 `.part` 與 SHA-256 manifest 防止半成品切換。
- 外部下載不使用帳號 Cookie；YouTube 回退只針對明確 gate 且有限次數。
- 文件與 commit 使用環境變數路徑，不寫入實際使用者名稱、私人 OneDrive 或 token。

## Git、同步與事件邊界

禁止提交 API Key、Cookie、影片、模型快取、原始影音、log、私人同步內容與未審查外部回應。若發現疑似 secret，立即停止發布、從 staged set 移除並依憑證輪替流程處理；不要只刪 Git 文字後宣稱已安全。
