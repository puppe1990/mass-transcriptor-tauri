import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  downloadBatchTranscriptsZip,
  getBatch,
  getTranscriptMarkdown,
  retryJob,
  type BatchDetail,
  type JobDetail,
} from "../lib/api";
import { JobStatusBadge } from "../components/JobStatusBadge";
import { IconChevronLeft } from "../components/icons";

type Props = {
  batchId: number;
  onBack: () => void;
  onOpenJob: (id: number) => void;
};

export function BatchPage({ batchId, onBack, onOpenJob }: Props) {
  const [batch, setBatch] = useState<BatchDetail | null>(null);
  const [activeJobId, setActiveJobId] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<number | null>(null);
  const [downloadBusy, setDownloadBusy] = useState(false);
  const [copyState, setCopyState] = useState<"idle" | "copied" | "failed">("idle");

  const refresh = useCallback(async () => {
    try {
      const detail = await getBatch(batchId);
      setBatch(detail);
      setError(null);
      if (detail && detail.jobs.length > 0) {
        setActiveJobId((current) => {
          if (current != null && detail.jobs.some((j) => j.id === current)) {
            return current;
          }
          return detail.jobs[0].id;
        });
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [batchId]);

  useEffect(() => {
    void refresh();
    const id = window.setInterval(() => void refresh(), 2000);
    let unlisten: (() => void) | undefined;
    void listen("job-updated", () => {
      void refresh();
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      window.clearInterval(id);
      unlisten?.();
    };
  }, [refresh]);

  const activeJob: JobDetail | null =
    batch?.jobs.find((j) => j.id === activeJobId) ?? batch?.jobs[0] ?? null;

  const canDownloadAll =
    batch?.jobs.some((j) => j.status === "completed" && j.markdownPath) ?? false;

  async function handleRetry(jobId: number) {
    setBusyId(jobId);
    try {
      await retryJob(jobId);
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusyId(null);
    }
  }

  async function handleDownloadOne(job: JobDetail) {
    try {
      const md = await getTranscriptMarkdown(job.id);
      triggerDownload(
        md,
        "text/markdown;charset=utf-8",
        `${job.originalFilename.replace(/\.[^.]+$/, "") || "transcript"}.md`,
      );
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  async function handleDownloadAll() {
    setDownloadBusy(true);
    setError(null);
    try {
      const b64 = await downloadBatchTranscriptsZip(batchId);
      const binary = atob(b64);
      const bytes = new Uint8Array(binary.length);
      for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
      triggerDownload(
        bytes,
        "application/zip",
        `batch-${batchId}-transcripts.zip`,
      );
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setDownloadBusy(false);
    }
  }

  async function handleCopy(text: string) {
    if (!navigator.clipboard) {
      setCopyState("failed");
      return;
    }
    try {
      await navigator.clipboard.writeText(text);
      setCopyState("copied");
      window.setTimeout(() => setCopyState("idle"), 2000);
    } catch {
      setCopyState("failed");
    }
  }

  if (!batch && !error) {
    return (
      <section className="page">
        <p className="page__subtitle">Loading…</p>
      </section>
    );
  }

  if (!batch) {
    return (
      <section className="page">
        <header className="page__header">
          <button type="button" className="page__back" onClick={onBack}>
            <IconChevronLeft />
            Back to jobs
          </button>
        </header>
        <div className="page-alert" role="alert">
          {error || "Batch not found"}
        </div>
      </section>
    );
  }

  return (
    <section className="page" id="batch-detail-page">
      <header className="page__header">
        <button type="button" className="page__back" onClick={onBack}>
          <IconChevronLeft />
          Back to jobs
        </button>
        <h1 className="page__title">Upload group · {batch.jobs.length} audios</h1>
        <p className="page__subtitle">Each file is listed below — switch tabs to review separately</p>
        {canDownloadAll && (
          <div className="page__actions">
            <button
              type="button"
              className="btn btn--primary"
              id="batch-download-all"
              disabled={downloadBusy}
              onClick={() => void handleDownloadAll()}
            >
              {downloadBusy ? "Preparing…" : "Download all"}
            </button>
          </div>
        )}
      </header>

      <div className="page__body">
        {error && (
          <div className="page-alert" role="alert">
            {error}
          </div>
        )}

        <div className="job-batch-tabs" role="tablist" aria-label="Batch files" id="batch-jobs-list">
          {batch.jobs.map((job) => (
            <button
              type="button"
              key={job.id}
              id={`batch-tab-${job.id}`}
              role="tab"
              aria-selected={job.id === activeJob?.id}
              className={[
                "job-batch-tabs__tab",
                job.id === activeJob?.id && "job-batch-tabs__tab--active",
              ]
                .filter(Boolean)
                .join(" ")}
              onClick={() => setActiveJobId(job.id)}
            >
              <span className="job-batch-tabs__label">{job.originalFilename}</span>
              <JobStatusBadge status={job.status} />
            </button>
          ))}
        </div>

        {activeJob && (
          <div
            className="job-batch-panel"
            role="tabpanel"
            id={`batch-panel-${activeJob.id}`}
          >
            <div className="job-meta" id={`job-meta-${activeJob.id}`}>
              <div className="job-meta__item">
                <p className="job-meta__label">File</p>
                <p className="job-meta__value">{activeJob.originalFilename}</p>
              </div>
              <div className="job-meta__item">
                <p className="job-meta__label">Provider</p>
                <p className="job-meta__value">{activeJob.providerKey}</p>
              </div>
              <div className="job-meta__item">
                <p className="job-meta__label">Status</p>
                <p className="job-meta__value">
                  <JobStatusBadge status={activeJob.status} />
                </p>
              </div>
              {activeJob.markdownPath && (
                <div className="job-meta__item">
                  <p className="job-meta__label">Output</p>
                  <p className="job-meta__value job-meta__value--muted">
                    {activeJob.markdownPath}
                  </p>
                </div>
              )}
            </div>

            {activeJob.errorMessage && (
              <p className="page-alert" role="status">
                {activeJob.errorMessage}
              </p>
            )}

            <div className="job-actions" id={`job-actions-${activeJob.id}`}>
              {activeJob.retryable && (
                <button
                  type="button"
                  className="btn btn--secondary"
                  disabled={busyId === activeJob.id}
                  onClick={() => void handleRetry(activeJob.id)}
                >
                  {busyId === activeJob.id ? "Retrying..." : "Retry job"}
                </button>
              )}
              {activeJob.status === "completed" && activeJob.markdownPath && (
                <button
                  type="button"
                  className="btn btn--primary"
                  onClick={() => void handleDownloadOne(activeJob)}
                >
                  Download markdown
                </button>
              )}
              <button
                type="button"
                className="btn btn--ghost"
                onClick={() => onOpenJob(activeJob.id)}
              >
                Open full page
              </button>
            </div>

            {!activeJob.transcriptText ? (
              <p className="transcript-preview__empty">No transcript yet.</p>
            ) : (
              <section className="transcript-preview">
                <div className="transcript-preview__header">
                  <h2>Transcript</h2>
                  <button
                    type="button"
                    className="transcript-preview__copy"
                    onClick={() => void handleCopy(activeJob.transcriptText!)}
                  >
                    {copyState === "copied" ? "Copied" : "Copy Text"}
                  </button>
                </div>
                {copyState === "failed" && (
                  <p className="transcript-preview__status" role="status">
                    Could not copy text.
                  </p>
                )}
                <pre>{activeJob.transcriptText}</pre>
              </section>
            )}
          </div>
        )}
      </div>
    </section>
  );
}

function triggerDownload(
  data: string | Uint8Array,
  mime: string,
  filename: string,
) {
  const blob =
    typeof data === "string"
      ? new Blob([data], { type: mime })
      : new Blob([new Uint8Array(data)], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}
