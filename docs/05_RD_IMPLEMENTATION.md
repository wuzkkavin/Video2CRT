# Video2CRT RD 研究與實作規格

- Audience: RD、維護者與接手 Agent
- Authority: current
- Status: 已驗證／部分研究項目未驗證
- Last verified: 2026-09-18
- Evidence: Git history、`DEVELOPMENT_GUIDE.md`、測試、發布腳本與模型 manifest

## 已完成研究與實作

- 以 oEmbed 取得公開標題，避免下載前額外 yt-dlp 標題請求與 ID 資料夾漂移。
- 對 PO Token／Visitor Data／429 只做一次 embedded client 回退，避免擴大登入或 Cookie 邊界。
- 以 cropdetect 提供影片專屬提示，保留手動 crop 覆寫，避免全域裁切破壞其他片源。
- 以 Python sidecar 實作 ASR、字幕語言／時間完整性判斷、OpenCC、翻譯清理與 burn guard。
- 以 NSIS installer hook 呼叫 bundled Python、固定 revision、SHA-256 manifest 實作安裝時高品質模型安裝。
- 以 Credential Manager 儲存雲端翻譯 Key，避免設定檔與 log 洩漏。

## 實作順序

1. 先建立或確認 `REQ-*` 與驗收條件。
2. 變更一層責任：UI、Rust 協調器、Python sidecar、模型管理不可混改無關行為。
3. 先跑窄測試，再跑 Rust／Python／前端完整回歸。
4. 需要影片時使用全新輸出資料夾，逐段比對 source、raw、subtitled、final。
5. 發布變更要重建 runtime staging、安裝包、hash 與 silent install。

## 延後研究

- 乾淨 Windows 使用者帳戶的完整 GUI → `final.mp4` E2E。
- 3.2 GB 高品質模型實際完整下載、取消後重啟與斷線續作。
- 多 DPI、長片、多語言與無 NVIDIA GPU 的完整視覺基準。

這些項目在證據取得前標示未驗證，不得寫成已完成。
