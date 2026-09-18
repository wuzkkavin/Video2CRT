# Video2CRT 測試、QA 與驗收規格

- Audience: 測試、RD、發布與接手 Agent
- Authority: current
- Status: 已驗證，部分完整 E2E 未驗證
- Last verified: 2026-09-18
- Evidence: `video2crt-app/TEST_ACCEPTANCE.md`、測試輸出與命令結果

## 測試矩陣

| ID | 層級 | 命令／證據 | 結果 |
| --- | --- | --- | --- |
| TST-001 | gotcha／依賴 | `python scripts/install_skill.py` | 38 條，`[ALL PASS]` |
| TST-002 | 根目錄 Python | `PYTHONPATH=src; python tests/run_all.py` | 13 項通過 |
| TST-003 | APP Python | `PYTHONPATH=..\\src; python scripts/test_subtitle_contract.py` | 26 項通過 |
| TST-004 | Rust | `cargo test --all-targets -- --test-threads=1` | 12 項通過 |
| TST-005 | 前端 | `npm run build` | 通過 |
| TST-006 | 發布 | `build-distributable.ps1`、silent install | 通過 |
| TST-007 | 完整 GUI／視覺 E2E | 乾淨帳戶、真實短片、畫面抽樣 | 未驗證 |
| TST-008 | 高品質模型 | 實際完整約 3.2 GB 下載與取消重啟 | 未驗證 |

## 驗收標準

`final.mp4` 必須有視訊與原始音訊；字幕 SRT 時間合法；中文單行、非中文雙行；翻譯缺漏不得進 burn；模型半成品不得啟用；安裝包不依賴開發機 checkout。結構檢查、單元測試、行為 E2E 與視覺 QA 必須分開記錄。

## 測試資料與安全

使用去識別化 fixture、公開短片與新輸出資料夾；不使用真實 API Key、Cookie、登入狀態、私人影片或未審查 log。雲端翻譯測試只記錄「已設定／未設定」與非敏感結果。

## 變更對應

字幕／翻譯改動至少跑 TST-002、TST-003、TST-004；UI 改動跑 TST-005 與對應互動驗收；發布腳本改動跑 TST-005、TST-006；模型管理改動跑 TST-004，真實下載另列未驗證或證據。
