/**
 * DonePage — final page of the wizard.
 *
 * Shown after `pipeline://done` arrives. Displays:
 *   - a green check + success heading
 *   - the output directory (clickable text — but no fancy open)
 *   - the final mp4 path
 *   - the srt path
 *   - "在 Explorer 開啟" → calls reveal_in_explorer(outputDir)
 *   - "新轉一個" → resets App back to the url page
 */

import { revealInExplorer } from "../lib/tauri";
import type { DoneEvent } from "../lib/types";

export interface DonePageProps {
  result: DoneEvent;
  onRestart: () => void;
}

export function DonePage({ result, onRestart }: DonePageProps) {
  const handleReveal = async () => {
    try {
      await revealInExplorer(result.outputDir);
    } catch (e: unknown) {
      // Fail silently in the UI; the user can copy the path manually.
      // Logging to console so the developer can see what went wrong.
      // eslint-disable-next-line no-console
      console.error("reveal_in_explorer failed:", e);
    }
  };

  return (
    <div className="done-page">
      <div className="done-banner">
        <div className="done-checkmark">✓</div>
        <h2>轉檔完成</h2>
        <p>已輸出 CRT 風格 MP4 + 燒錄字幕。</p>
      </div>

      <div className="done-paths">
        <div className="done-path">
          <span className="label">輸出資料夾</span>
          <span className="value">{result.outputDir}</span>
        </div>
        <div className="done-path">
          <span className="label">最終影片</span>
          <span className="value">{result.finalMp4}</span>
        </div>
        <div className="done-path">
          <span className="label">字幕檔</span>
          <span className="value">{result.srt}</span>
        </div>
      </div>

      <div className="done-actions">
        <button className="btn" onClick={() => void handleReveal()}>
          📁 在 Explorer 開啟
        </button>
        <button className="btn btn-primary" onClick={onRestart}>
          新轉一個 →
        </button>
      </div>
    </div>
  );
}
