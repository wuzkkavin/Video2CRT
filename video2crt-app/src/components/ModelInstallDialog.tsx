import type { LargeModelStatus, ModelInstallProgress } from "../lib/types";

export interface ModelInstallDialogProps {
  open: boolean;
  status: LargeModelStatus | null;
  progress: ModelInstallProgress | null;
  busy: boolean;
  error: string | null;
  onInstall: () => void;
  onUseStandard: () => void;
  onCancel: () => void;
  onCancelDownload: () => void;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(0)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GB`;
}

export function ModelInstallDialog({
  open,
  status,
  progress,
  busy,
  error,
  onInstall,
  onUseStandard,
  onCancel,
  onCancelDownload,
}: ModelInstallDialogProps) {
  if (!open) return null;
  const overall = progress?.stage === "complete" ? 1 : progress && progress.totalBytes > 0
    ? Math.min(0.99, progress.downloadedBytes / progress.totalBytes)
    : 0;
  return (
    <div className="dialog-backdrop" role="presentation">
      <section className="dialog model-install-dialog" role="dialog" aria-modal="true" aria-labelledby="model-install-title">
        <div className="dialog-header">
          <h3 id="model-install-title">安裝高品質本機模型</h3>
          {!busy ? <button className="dialog-close" onClick={onCancel} aria-label="關閉">×</button> : null}
        </div>
        <div className="dialog-body">
          <p className="dialog-copy">
            Video2CRT 已內建可離線使用的標準模型。高品質模型會改善語音辨識與翻譯結果，約需下載 3.2 GB，並需要網路與足夠的磁碟空間。
          </p>
          <div className="model-install-facts">
            <span>標準模型：立即可用</span>
            <span>高品質模型：約 {formatBytes(status?.estimatedBytes ?? 3_200_000_000)}</span>
          </div>
          {busy ? (
            <div className="model-install-progress" aria-live="polite">
              <div className="progress-track">
                <div className="progress-fill" style={{ width: `${Math.round(overall * 100)}%` }} />
              </div>
              <div className="model-install-progress-meta">
                <span>{progress?.message ?? "準備下載…"}</span>
                <span>{Math.round(overall * 100)}%</span>
              </div>
              <div className="dialog-note">
                已下載 {formatBytes(progress?.downloadedBytes ?? 0)}。完成驗證後才會啟用模型。
              </div>
            </div>
          ) : null}
          {error ? <div className="error-banner">{error}</div> : null}
          <p className="dialog-note">
            模型會安裝到目前 Windows 使用者的 Video2CRT 資料夾，不會修改系統 Python 或其他全域元件。
          </p>
        </div>
        <div className="dialog-footer">
          {busy ? (
            <button className="btn" onClick={onCancelDownload}>取消下載</button>
          ) : (
            <button className="btn" onClick={onUseStandard}>使用標準模型</button>
          )}
          <div className="right">
            {!busy ? <button className="btn btn-primary" onClick={onInstall}>安裝高品質模型</button> : null}
          </div>
        </div>
      </section>
    </div>
  );
}
