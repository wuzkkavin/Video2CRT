/**
 * SettingsDialog — manage the cloud-translation API key.
 *
 * Behaviour:
 *   * On open, fetches `has_api_key()` and `list_translation_models()` in
 *     parallel so the status badge and model list render with real data.
 *   * "Save" persists the current input through `save_api_key(...)` and
 *     flips the badge to "已儲存".
 *   * "Delete" clears the stored key via `delete_api_key()`.
 *   * The key input is uncontrolled-ish: it always reflects what the user
 *     typed (not the masked server state), but a fresh placeholder is
 *     shown when a key is already saved so the user can decide whether to
 *     replace or delete it.
 *
 * Closing the dialog just hides it — there is no "apply" step because
 * save/delete are themselves atomic. The parent decides when to reopen.
 */

import { useEffect, useRef, useState } from "react";
import {
  deleteApiKey,
  hasApiKey,
  listTranslationModels,
  saveApiKey,
} from "../lib/tauri";
import type { ModelInfo } from "../lib/types";

export interface SettingsDialogProps {
  open: boolean;
  onClose: () => void;
}

export function SettingsDialog({ open, onClose }: SettingsDialogProps) {
  const [hasKey, setHasKey] = useState(false);
  const [keyInput, setKeyInput] = useState("");
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  // Track whether the input is dirty so we can disable Save until needed.
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    setLoading(true);
    setError(null);
    setKeyInput("");
    Promise.all([hasApiKey(), listTranslationModels()])
      .then(([keyPresent, modelList]) => {
        if (cancelled) return;
        setHasKey(keyPresent);
        setModels(modelList);
      })
      .catch((e: unknown) => {
        if (cancelled) return;
        setError(e instanceof Error ? e.message : String(e));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [open]);

  if (!open) return null;

  const handleSave = async () => {
    const trimmed = keyInput.trim();
    if (!trimmed) {
      setError("請先輸入 API key");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await saveApiKey(trimmed);
      setHasKey(true);
      setKeyInput("");
      // Refresh models in case the new key unlocks the live list.
      try {
        const fresh = await listTranslationModels();
        setModels(fresh);
      } catch {
        /* non-fatal */
      }
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleDelete = async () => {
    setBusy(true);
    setError(null);
    try {
      await deleteApiKey();
      setHasKey(false);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const canSave = keyInput.trim().length > 0 && !busy;

  return (
    <div
      className="dialog-backdrop"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
      role="dialog"
      aria-modal="true"
      aria-label="設定"
    >
      <div className="dialog">
        <div className="dialog-header">
          <h3>設定</h3>
          <button
            className="btn btn-ghost"
            onClick={onClose}
            aria-label="關閉"
          >
            ✕
          </button>
        </div>

        <div className="dialog-body">
          <div className="field">
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
              }}
            >
              <span className="field-label">Minimax API Key</span>
              <span className={`key-status ${hasKey ? "saved" : "unsaved"}`}>
                <span className="dot" />
                {loading ? "讀取中…" : hasKey ? "已儲存" : "未儲存"}
              </span>
            </div>
            <input
              ref={inputRef}
              className="input"
              type="password"
              autoComplete="new-password"
              spellCheck={false}
              placeholder={
                hasKey ? "已儲存（輸入新值可覆蓋）" : "貼上你的 API key"
              }
              value={keyInput}
              onChange={(e) => setKeyInput(e.target.value)}
              disabled={busy}
              onKeyDown={(e) => {
                if (e.key === "Enter" && canSave) void handleSave();
              }}
            />
            <span className="field-hint">
              用途：雲端字幕翻譯（選填）。影片轉檔與本機翻譯不需要 Key。
            </span>
          </div>

          <div className="field">
            <span className="field-label">翻譯模型清單</span>
            <div className="models-list">
              {loading ? (
                <div className="models-list-empty">載入中…</div>
              ) : models.length === 0 ? (
                <div className="models-list-empty">沒有可用模型</div>
              ) : (
                models.map((m) => (
                  <div
                    key={m.id}
                    className={`model-row ${m.isDefault ? "default" : ""}`}
                  >
                    <div>
                      <span className="id">{m.id}</span>
                      {m.isDefault ? (
                        <span className="badge">DEFAULT</span>
                      ) : null}
                      {m.label && m.label !== m.id ? (
                        <span className="meta"> — {m.label}</span>
                      ) : null}
                    </div>
                    <span className="meta">{m.group}</span>
                  </div>
                ))
              )}
            </div>
            <span className="field-hint">
              載入時會先用已儲存的 key 試 GET
              /v1/models，失敗時 fallback 為硬編碼清單。
            </span>
          </div>

          {error ? <div className="error-banner">{error}</div> : null}
        </div>

        <div className="dialog-footer">
          <div className="left">
            <button
              className="btn btn-danger"
              onClick={() => void handleDelete()}
              disabled={!hasKey || busy}
            >
              刪除
            </button>
          </div>
          <div className="right">
            <span className="dialog-note">
              API Key 會加密儲存在 Windows Credential Manager；重新開啟時只顯示是否已設定，不會把內容載回畫面。
            </span>
            <button
              className="btn btn-primary"
              onClick={() => void handleSave()}
              disabled={!canSave}
            >
              儲存
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
