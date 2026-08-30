import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  cancelJob,
  getJob,
  getTranscriptMarkdown,
  retryJob,
  type JobDetail,
} from "../lib/api";
import { JobStatusBadge } from "../components/JobStatusBadge";
import { IconChevronLeft } from "../components/icons";
import { useLocale } from "../lib/LocaleContext";

type Props = {
  jobId: number;
  onBack: () => void;
  onOpenBatch: (id: number) => void;
};

export function JobPage({ jobId, onBack, onOpenBatch }: Props) {
  const { t } = useLocale();
  const [job, setJob] = useState<JobDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [copyState, setCopyState] = useState<"idle" | "copied" | "failed">("idle");

  const refresh = useCallback(async () => {
    try {
      const detail = await getJob(jobId);
      setJob(detail);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [jobId]);

  useEffect(() => {
    void refresh();
    const id = window.setInterval(() => void refresh(), 2000);
    let unlisten: (() => void) | undefined;
    void listen("job-updated", (ev) => {
      const payload = ev.payload as { jobId?: number };
      if (payload?.jobId === jobId) void refresh();
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      window.clearInterval(id);
      unlisten?.();
    };
  }, [jobId, refresh]);

  async function handleRetry() {
    setBusy(true);
    try {
      await retryJob(jobId);
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function handleCancel() {
    setBusy(true);
    try {
      await cancelJob(jobId);
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function handleDownload() {
    try {
      const md = await getTranscriptMarkdown(jobId);
      const blob = new Blob([md], { type: "text/markdown;charset=utf-8" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${job?.originalFilename?.replace(/\.[^.]+$/, "") || "transcript"}.md`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  async function handleCopy() {
    if (!job?.transcriptText || !navigator.clipboard) {
      setCopyState("failed");
      return;
    }
    try {
      await navigator.clipboard.writeText(job.transcriptText);
      setCopyState("copied");
      window.setTimeout(() => setCopyState("idle"), 2000);
    } catch {
      setCopyState("failed");
    }
  }

  if (!job && !error) {
    return (
      <section className="page">
        <p className="page__subtitle">{t("job.loading")}</p>
      </section>
    );
  }

  if (!job) {
    return (
      <section className="page">
        <header className="page__header">
          <button type="button" className="page__back" onClick={onBack}>
            <IconChevronLeft />
            {t("job.back")}
          </button>
        </header>
        <div className="page-alert" role="alert">
          {error || t("job.notFound")}
        </div>
      </section>
    );
  }

  return (
    <section className="page" id="job-detail-page">
      <header className="page__header">
        <button type="button" className="page__back" id="job-back" onClick={onBack}>
          <IconChevronLeft />
          {t("job.back")}
        </button>
        <h1 className="page__title">{job.originalFilename}</h1>
        {job.batchId != null && (
          <div className="page__actions">
            <a
              href={`#batch-${job.batchId}`}
              onClick={(e) => {
                e.preventDefault();
                onOpenBatch(job.batchId!);
              }}
            >
              {t("job.batch", { id: job.batchId })}
            </a>
          </div>
        )}
      </header>

      <div className="page__body">
        {error && (
          <div className="page-alert" role="alert">
            {error}
          </div>
        )}

        <div className="job-meta" id={`job-meta-${job.id}`}>
          <div className="job-meta__item">
            <p className="job-meta__label">{t("job.provider")}</p>
            <p className="job-meta__value">{job.providerKey}</p>
          </div>
          <div className="job-meta__item">
            <p className="job-meta__label">{t("job.status")}</p>
            <p className="job-meta__value">
              <JobStatusBadge status={job.status} />
            </p>
          </div>
          {job.markdownPath && (
            <div className="job-meta__item">
              <p className="job-meta__label">{t("job.output")}</p>
              <p className="job-meta__value job-meta__value--muted">{job.markdownPath}</p>
            </div>
          )}
        </div>

        {job.errorMessage && (
          <p className="page-alert" id="job-error" role="status">
            {job.errorMessage}
          </p>
        )}

        <div className="job-actions" id={`job-actions-${job.id}`}>
          {job.status === "processing" && (
            <button
              type="button"
              className="btn btn--ghost"
              id="cancel-job-btn"
              disabled={busy}
              onClick={() => void handleCancel()}
            >
              {busy ? t("job.cancelling") : t("job.cancel")}
            </button>
          )}
          {job.retryable && (
            <button
              type="button"
              className="btn btn--secondary"
              id="retry-job-btn"
              disabled={busy}
              onClick={() => void handleRetry()}
            >
              {busy ? t("job.retrying") : t("job.retry")}
            </button>
          )}
          {job.status === "completed" && job.markdownPath && (
            <button
              type="button"
              className="btn btn--primary"
              id="download-md-btn"
              onClick={() => void handleDownload()}
            >
              {t("job.downloadMd")}
            </button>
          )}
        </div>

        {!job.transcriptText ? (
          <p className="transcript-preview__empty" id="transcript-empty">
            {t("job.empty")}
          </p>
        ) : (
          <section className="transcript-preview" id="transcript-preview">
            <div className="transcript-preview__header">
              <h2>{t("job.manuscript")}</h2>
              <button
                type="button"
                id="transcript-copy"
                className="transcript-preview__copy"
                onClick={() => void handleCopy()}
              >
                {copyState === "copied" ? t("job.copied") : t("job.copy")}
              </button>
            </div>
            {copyState === "failed" && (
              <p id="transcript-copy-status" className="transcript-preview__status" role="status">
                {t("job.copyFailed")}
              </p>
            )}
            <pre id="transcript-text">{job.transcriptText}</pre>
          </section>
        )}
      </div>
    </section>
  );
}
