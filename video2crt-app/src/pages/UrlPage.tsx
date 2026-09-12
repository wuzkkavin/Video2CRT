/**
 * UrlPage — first page of the wizard.
 *
 * Single input for a YouTube (or otherwise yt-dlp-compatible) URL, a
 * handful of pre-filled examples, and a "下一步" button that hands the
 * value to App via `onSubmit`. Validation is intentionally permissive —
 * the orchestrator derives a video id from whatever the user pastes, so
 * empty / malformed input is rejected here but anything non-empty goes
 * through.
 */

import { useState, type FormEvent } from "react";

export interface UrlPageProps {
  initialUrl: string;
  onSubmit: (url: string) => void;
}

const EXAMPLE_URLS: ReadonlyArray<{ label: string; url: string }> = [
  {
    label: "YouTube 範例",
    url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
  },
  {
    label: "MV",
    url: "https://www.youtube.com/watch?v=jfKfPfyJRdk",
  },
  {
    label: "youtu.be 短網址",
    url: "https://youtu.be/dQw4w9WgXcQ",
  },
];

export function UrlPage({ initialUrl, onSubmit }: UrlPageProps) {
  const [url, setUrl] = useState(initialUrl);

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    const trimmed = url.trim();
    if (!trimmed) return;
    onSubmit(trimmed);
  };

  return (
    <form className="url-page" onSubmit={handleSubmit}>
      <div className="url-hero">
        <h1>
          Video2<span className="accent">CRT</span>
        </h1>
        <p>
          貼上 YouTube 連結，一鍵轉成 CRT 螢光幕風格的 MP4。內建
          libplacebo CRT shader + faster-whisper 字幕。
        </p>
      </div>

      <div className="field">
        <label className="field-label" htmlFor="url-input">
          影片網址
        </label>
        <div className="url-input-row">
          <input
            id="url-input"
            className="input"
            type="url"
            placeholder="https://www.youtube.com/watch?v=…"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            autoFocus
            spellCheck={false}
          />
          <button
            className="btn btn-primary"
            type="submit"
            disabled={url.trim().length === 0}
          >
            下一步 →
          </button>
        </div>
      </div>

      <div className="field" style={{ marginBottom: 0 }}>
        <span className="field-label">快速填入</span>
        <div className="url-examples">
          {EXAMPLE_URLS.map((ex) => (
            <button
              key={ex.url}
              type="button"
              className="url-example"
              onClick={() => setUrl(ex.url)}
              title={ex.url}
            >
              {ex.label}
            </button>
          ))}
        </div>
      </div>
    </form>
  );
}
