# Video2CRT 決策與需求追蹤

- Audience: 產品、SA、SD、RD、測試與接手 Agent
- Authority: current decisions plus historical links
- Status: 已驗證
- Last verified: 2026-09-18
- Evidence: `SPEC.md`、`DEVELOPMENT_GUIDE.md`、Git commit、測試矩陣

## 主要決策

| ID | 決策 | 原因／替代方案 | 影響 |
| --- | --- | --- | --- |
| ADR-001 | 主包內放標準模型，高品質模型由安裝程序確認後下載 | NSIS 單檔限制與下載透明度；不採運行時提示或靜默下載 | `installer-hooks.nsh`、manifest |
| ADR-002 | current-user NSIS + offline WebView2 | 一般電腦不需管理員與網路 WebView2 | 安裝目錄自包含，需重建 runtime |
| ADR-003 | ASR／本機翻譯留在 Python sidecar 本機執行 | 隱私與離線能力；不把影片上傳雲端 | runtime 需打包 Python 依賴與模型 |
| ADR-004 | YouTube access gate 只有限回退 embedded client | 保留成功路徑，不使用 Cookie／登入 | 服務端行為漂移時需新增證據 |
| ADR-005 | 保留既有 CRT、crop、NVENC CQ 23 基準 | 字幕修正不應改變已認可畫質 | UI／字幕變更需跑畫質與格式驗收 |
| ADR-006 | 無伺服器 DB，採 Credential Manager、模型 manifest、工作檔 | 目前是本機桌面工具 | 未來若同步或帳號化必須新增 migration 設計 |

## Traceability matrix

| Requirement | Design | Implementation | Test／evidence |
| --- | --- | --- | --- |
| REQ-001 | DES-001 self-contained runtime | `build-distributable.ps1`, Tauri config | TST-005／TST-006 |
| REQ-002 | DES-002 UI wizard | `UrlPage`, `OptionsPage` | TST-005；GUI E2E 未驗證 |
| REQ-003 | DES-003 output allocator | `orchestrator.rs` | Rust unit tests |
| REQ-004 | DES-004 progress events | `lib.rs`, `ProgressPage` | build；完整 GUI 未驗證 |
| REQ-005 | DES-005 subtitle engine | `pipeline_cli.py`, `subtitle_engine.py` | TST-002／TST-003／TST-004 |
| REQ-006 | DES-006 model lifecycle | `installer-hooks.nsh`, `install-large-model.py`, `model_manager.rs` | Rust model test；3.2 GB 實下載未驗證 |
| REQ-007 | DES-007 burn guard | Python pipeline | TST-003 |
| REQ-008 | DES-008 secret boundary | `settings.rs` | code review、敏感掃描 |
| REQ-009 | DES-009 media contract | ffmpeg pipeline | 字幕契約；完整視覺 E2E 未驗證 |
| REQ-010 | DES-010 release evidence | build script、RELEASE | installer hash、silent install |

任何新需求先增加 `REQ-*`，再補 `DES-*`、實作檔案、`TST-*` 與發布證據；不要只在聊天紀錄中保存決策。
