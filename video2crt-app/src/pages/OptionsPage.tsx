/**
 * OptionsPage — second page of the wizard.
 *
 * Shows the URL the user just submitted and lets them tweak:
 *   - crop value (default empty → Rust auto-detects via cropdetect,
 *     gotcha 6 + 24 + 34)
 *   - ASR language (auto / ja / en / zh)
 *   - cloud translation toggle + model (disabled when toggle is off)
 *
 * The Settings button opens the dialog. "開始轉檔" kicks the pipeline.
 */

import { useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { hasApiKey } from "../lib/tauri";
import type { JobOptions } from "../lib/types";

export interface OptionsPageProps {
  url: string;
  onBack: () => void;
  onStart: (opts: JobOptions) => Promise<void>;
  onOpenSettings: () => void;
  defaultOptions: JobOptions;
}

export function OptionsPage({
  url,
  onBack,
  onStart,
  onOpenSettings,
  defaultOptions,
}: OptionsPageProps) {
  const [crop, setCrop] = useState(defaultOptions.crop);
  const [asrLanguage, setAsrLanguage] =
    useState<JobOptions["asrLanguage"]>(defaultOptions.asrLanguage);
  const [subtitleMode, setSubtitleMode] = useState<JobOptions["subtitleMode"]>(
    defaultOptions.subtitleMode,
  );
  const [translationMode, setTranslationMode] = useState<JobOptions["translationMode"]>(
    defaultOptions.translationMode,
  );
  const [translationModel, setTranslationModel] = useState(
    defaultOptions.translationModel,
  );
  // Output directory. Empty string means "use the default
  // <Desktop>/<video title>/". When the user picks a folder via
  // the dialog, we store its absolute path here and ship it to Rust.
  const [outputDir, setOutputDir] = useState(defaultOptions.outputDir);

  // Local in-flight guard. Once the user clicks 開始轉檔, we lock the
  // button immediately so a double-tap can't fire a second
  // `start_job` IPC — which would spawn a second `yt-dlp`/ffmpeg/ASR
  // pipeline against the same output dir, corrupting `source.mp4`
  // and crashing the GPU encoder (`0xC0000005`).
  const [starting, setStarting] = useState(false);

  const handleStart = async () => {
    if (starting) return;
    setStarting(true);
    try { await onStart({
      crop: crop.trim(),
      asrLanguage,
      enableSubtitles: subtitleMode !== "none",
      subtitleMode,
      translationMode,
      cloudTranslation: subtitleMode === "bilingual" && (translationMode === "cloudFallback" || translationMode === "cloud"),
      translationModel: subtitleMode === "bilingual" && (translationMode === "cloudFallback" || translationMode === "cloud") ? translationModel : "",
      outputDir: outputDir.trim(),
    }); } finally { setStarting(false); }
  };

  const chooseTranslationMode = async (mode: JobOptions["translationMode"]) => {
    setTranslationMode(mode);
    if ((mode === "cloud" || mode === "cloudFallback") && !(await hasApiKey())) {
      onOpenSettings();
    }
  };

  // Pop the OS folder picker via Tauri's dialog plugin. Returns
  // absolute path or null if the user cancelled.
  const handlePickOutputDir = async () => {
    try {
      const result = await openDialog({
        directory: true,
        multiple: false,
        title: "選擇輸出資料夾",
        defaultPath: outputDir || undefined,
      });
      if (typeof result === "string") {
        setOutputDir(result);
      }
    } catch (e) {
      // Silent: the dialog can throw if the user closes it via the
      // X button before the promise resolves; not actionable.
      console.warn("folder picker cancelled", e);
    }
  };

  return (
    <div className="options-page">
      <h2>轉檔選項</h2>
      <div className="options-url-preview" title={url}>
        {url}
      </div>

      <div className="field">
        <label className="field-label" htmlFor="crop-input">
          裁切參數 (crop)
        </label>
        <input
          id="crop-input"
          className="input"
          type="text"
          value={crop}
          onChange={(e) => setCrop(e.target.value)}
          spellCheck={false}
          placeholder="自動偵測 (留空)"
        />
        <span className="field-hint">
          格式 W:H:X:Y。預設 <code>960:720:160:0</code>{" "}
          適用於 4:3 內容在 16:9 容器內（左右各切 160px）。
        </span>
      </div>

      <div className="field">
        <label className="field-label" htmlFor="asr-lang">
          ASR 語言
        </label>
        <select
          id="asr-lang"
          className="select"
          value={asrLanguage}
          onChange={(e) =>
            setAsrLanguage(e.target.value as JobOptions["asrLanguage"])
          }
          disabled={subtitleMode === "none"}
        >
          <option value="auto">自動偵測 (auto)</option>
          <option value="ja">日本語 (ja)</option>
          <option value="en">English (en)</option>
          <option value="zh">中文 (zh)</option>
        </select>
      </div>

      <div className="field">
        <label className="field-label">字幕輸出</label>
        <div className="radio-group" role="radiogroup" aria-label="字幕輸出">
          <label className="checkbox-row">
            <input type="radio" name="subtitle-mode" value="original"
              checked={subtitleMode === "original"}
              onChange={() => setSubtitleMode("original")} />
            只有原文字幕
          </label>
          <label className="checkbox-row">
            <input type="radio" name="subtitle-mode" value="bilingual"
              checked={subtitleMode === "bilingual"}
              onChange={() => setSubtitleMode("bilingual")} />
            原文＋繁體中文字幕
          </label>
          <label className="checkbox-row">
            <input type="radio" name="subtitle-mode" value="none"
              checked={subtitleMode === "none"}
              onChange={() => setSubtitleMode("none")} />
            無字幕
          </label>
        </div>
        <span className="field-hint">
          中文原片維持繁中單行；選擇無字幕會略過字幕辨識與燒錄。
        </span>
      </div>

      {subtitleMode === "bilingual" && (
        <div className="field">
          <label className="field-label">翻譯方式</label>
          <div className="radio-group" role="radiogroup" aria-label="翻譯方式">
            <label className="checkbox-row">
              <input type="radio" name="translation-mode" value="local"
                checked={translationMode === "local"}
                onChange={() => void chooseTranslationMode("local")} />
              本機端翻譯（預設）
            </label>
            <label className="checkbox-row">
              <input type="radio" name="translation-mode" value="cloudFallback"
                checked={translationMode === "cloudFallback"}
                onChange={() => void chooseTranslationMode("cloudFallback")} />
              本機端優先，失敗時使用雲端翻譯
            </label>
            <label className="checkbox-row">
              <input type="radio" name="translation-mode" value="cloud"
                checked={translationMode === "cloud"}
                onChange={() => void chooseTranslationMode("cloud")} />
              純雲端翻譯
            </label>
          </div>
          <span className="field-hint">
            本機端可翻譯時，文字不會傳出電腦。選擇雲端備援且已儲存 API Key 時，本機失敗才會把外語字幕送至 MiniMax。
          </span>

          <label className="field-label" htmlFor="translation-model">
            翻譯模型
          </label>
          <select
            id="translation-model"
            className="select"
            value={translationModel}
            onChange={(e) => setTranslationModel(e.target.value)}
            disabled={translationMode !== "cloudFallback" && translationMode !== "cloud"}
          >
            {translationMode === "cloudFallback" || translationMode === "cloud" ? null : <option value="">(請選擇雲端翻譯)</option>}
            <option value="MiniMax-M3">MiniMax-M3 (default, 1M ctx)</option>
            <option value="MiniMax-M2.7">MiniMax-M2.7</option>
            <option value="MiniMax-M2.7-highspeed">MiniMax-M2.7-highspeed</option>
            <option value="MiniMax-M2.5">MiniMax-M2.5 (legacy)</option>
            <option value="MiniMax-M2.5-highspeed">
              MiniMax-M2.5-highspeed (legacy)
            </option>
            <option value="MiniMax-M2.1">MiniMax-M2.1 (legacy)</option>
            <option value="MiniMax-M2">MiniMax-M2 (legacy)</option>
          </select>
          <span className="field-hint">
            雲端備援前，請先在「設定」中儲存 API key。
          </span>
        </div>
      )}

      <div className="field">
        <label className="field-label" htmlFor="output-dir-input">
          輸出資料夾
        </label>
        <div className="output-dir-row">
          <input
            id="output-dir-input"
            className="input"
            type="text"
            value={outputDir}
            onChange={(e) => setOutputDir(e.target.value)}
            spellCheck={false}
            placeholder="留空使用桌面／YouTube 影片標題／"
          />
          <button
            type="button"
            className="btn"
            onClick={handlePickOutputDir}
            title="瀏覽…"
          >
            瀏覽…
          </button>
        </div>
        <span className="field-hint">
          在所選位置建立「YouTube 影片標題」資料夾，存放影片與字幕；同名時自動加序號。留空時使用桌面。
        </span>
      </div>

      <div className="options-actions">
        <div style={{ display: "flex", gap: 8 }}>
          <button className="btn" onClick={onBack}>
            ← 返回
          </button>
          <button className="btn" onClick={onOpenSettings}>
            ⚙ 設定
          </button>
        </div>
        <button
          className="btn btn-primary"
          onClick={handleStart}
          disabled={starting}
          style={starting ? { opacity: 0.5, cursor: "not-allowed" } : undefined}
        >
          {starting ? "轉檔中…" : "開始轉檔 →"}
        </button>
      </div>
    </div>
  );
}
