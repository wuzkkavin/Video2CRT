# Video2CRT 完全開發設計文件索引

- Audience: 產品、SA、SD、UI/UX、RD、測試、維運與接手 Agent
- Authority: current documentation map
- Status: 已驗證（文件依目前程式碼、設定、測試與發布產物整理）
- Last verified: 2026-09-18
- Evidence: `video2crt-app/` 原始碼、manifest、測試輸出、安裝包與 Git history

## 一次閱讀入口

新開發者先讀本索引，再讀 [根目錄 HANDOFF](../HANDOFF.md)、[APP SPEC](../video2crt-app/SPEC.md) 與 [Agent 接手指南](../video2crt-app/AGENT_GUIDE.md)。本索引是角色文件地圖；完整流程與歷史狀態仍以各權威文件為準。

## 文件地圖

| 角色／責任 | 權威文件 | 狀態 |
| --- | --- | --- |
| 需求、範圍、驗收基準 | [01_REQUIREMENTS.md](01_REQUIREMENTS.md)；補充 [APP SPEC](../video2crt-app/SPEC.md) | 已驗證 |
| 系統分析 SA | [02_SYSTEM_ANALYSIS.md](02_SYSTEM_ANALYSIS.md) | 已驗證 |
| 軟體設計 SD | [03_SOFTWARE_DESIGN.md](03_SOFTWARE_DESIGN.md)；補充 [DEVELOPMENT_GUIDE](../video2crt-app/DEVELOPMENT_GUIDE.md) | 已驗證 |
| UI/UX | [04_UI_UX_SPEC.md](04_UI_UX_SPEC.md)；補充 [USER_GUIDE](../video2crt-app/USER_GUIDE.md) | 結構已驗證，完整視覺 E2E 未驗證 |
| RD／研究與實作 | [05_RD_IMPLEMENTATION.md](05_RD_IMPLEMENTATION.md) | 已驗證／部分未驗證 |
| 資料與 DB | [06_DATA_DB_SPEC.md](06_DATA_DB_SPEC.md) | 已驗證：本版本無伺服器 DB |
| API 與外部整合 | [07_API_INTEGRATIONS.md](07_API_INTEGRATIONS.md) | 已驗證 |
| 測試與 QA | [08_TEST_QA_ACCEPTANCE.md](08_TEST_QA_ACCEPTANCE.md)；補充 [TEST_ACCEPTANCE](../video2crt-app/TEST_ACCEPTANCE.md) | 已驗證，部分 E2E 未驗證 |
| 安全與資料處理 | [09_SECURITY_DATA.md](09_SECURITY_DATA.md)；補充 [DEVELOPMENT_GUIDE](../video2crt-app/DEVELOPMENT_GUIDE.md) | 已驗證 |
| 發布與維運 | [10_RELEASE_OPERATIONS.md](10_RELEASE_OPERATIONS.md)；補充 [RELEASE](../video2crt-app/RELEASE.md) | 已驗證，乾淨帳戶 E2E 未驗證 |
| 決策與追蹤 | [11_DECISIONS_TRACEABILITY.md](11_DECISIONS_TRACEABILITY.md) | 已驗證 |
| 操作與接手 | [USER_GUIDE](../video2crt-app/USER_GUIDE.md)、[AGENT_GUIDE](../video2crt-app/AGENT_GUIDE.md)、[HANDOFF](../HANDOFF.md) | 已驗證 |

## Traceability 基準

需求使用 `REQ-*`、設計使用 `DES-*`、測試使用 `TST-*`、決策使用 `ADR-*`。每個重要需求都應在 [11_DECISIONS_TRACEABILITY.md](11_DECISIONS_TRACEABILITY.md) 連到設計、程式來源與測試證據。

## 當前驗證

- gotcha 前置檢查：38 條，`scripts/install_skill.py` `[ALL PASS]`。
- 自動化測試：51 項（根目錄 13、字幕契約 26、Rust 12）。
- 前端建置：`npm run build` 通過。
- 發布包：`video2crt-app/dist-distributable/Video2CRT_0.1.0_x64-setup.exe` 已產生並完成 silent install 冒煙測試。

## 已知缺口

- 尚未在乾淨 Windows 使用者帳戶完成完整 GUI 到 `final.mp4` 的視覺 E2E。
- 尚未實際下載完整約 3.2 GB 高品質模型；只驗證模型管理與 manifest 邏輯。
- GitHub `origin` 目前是公開 repository；不要在文件或 commit 放入憑證、Cookie、使用者影片或本機私密資料。
