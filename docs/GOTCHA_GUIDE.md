# Video2CRT CRT skill gotcha 指南

## 唯一權威來源

Video2CRT 使用的 38 條 CRT gotcha 不在各個輸出資料夾分別維護。唯一權威原始檔是 Hermes 本機安裝的 skill：

```text
%LOCALAPPDATA%\hermes\skills\video-crt-geom-libplacebo\SKILL.md
```

實際使用者名稱與磁碟位置可能不同，請以 `%LOCALAPPDATA%` 展開後的路徑為準；不要把某台電腦的絕對使用者路徑寫入交接文件。

專案中的 `HANDOFF.md`、`video2crt-app/DEVELOPMENT_GUIDE.md`、程式註解、測試註解與 `output/*/handoff.md` 只保存摘要、實作對照或歷史驗收脈絡，不可當成第二份完整 skill，也不可從它們推算 gotcha 總數。

## 每次接手或開始影片工作前

從專案根目錄執行：

```powershell
python scripts/install_skill.py
```

必須看到：

```text
[OK] Found 38 gotchas in SKILL.md
[ALL PASS] Skill v36+ installed and dependencies OK.
```

`EXPECTED_GOTCHA_COUNT = 36` 是最低新鮮度門檻，不是目前總數。若實際數量低於 36、skill 不存在、或 ffmpeg／yt-dlp 依賴檢查失敗，先停止並回報環境，不要開始轉檔或修改 pipeline。

目前實測的編號為 `0–31、33–38`；32 號沒有對應條目。不可用最大編號當作總數，也不可自行補寫缺少的 32 號。

## 專案內的對照位置

| 用途 | 位置 | 說明 |
| --- | --- | --- |
| 完整規則 | `%LOCALAPPDATA%\\hermes\\skills\\video-crt-geom-libplacebo\\SKILL.md` | 唯一權威來源 |
| 安裝與計數檢查 | `scripts/install_skill.py` | 解析本機 skill、計算編號集合、檢查依賴 |
| 接手摘要 | `HANDOFF.md` | 只記錄目前版本基準與必要起手式 |
| APP 開發摘要 | `video2crt-app/DEVELOPMENT_GUIDE.md` | 只記錄與 APP 實作相關的 gotcha 對照 |
| 回歸測試 | `tests/`、`video2crt-app/scripts/test_subtitle_contract.py` | 驗證部分契約，不等於完整 gotcha 清單 |
| 影片個案紀錄 | `output/*/handoff.md` | 只屬於該次輸出，不可當全域規格 |

新增或修正 gotcha 時，先更新權威 skill，再更新本索引與 `HANDOFF.md` 的版本基準，最後補相應測試；不要只在某個 `output/*` 或程式註解裡新增規則。
