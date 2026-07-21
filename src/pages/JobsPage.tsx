import { useCallback, useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { listJobRows, listJobs, type JobListRow, type JobSummary } from "../lib/api";
import { JobStatusBadge } from "../components/JobStatusBadge";
import {
  IconChevronRight,
  IconFolder,
  IconMusic,
  IconUploadTray,
} from "../components/icons";

type Props = {
  onOpenJob: (id: number) => void;
  onOpenBatch: (id: number) => void;
  onOpenUpload: () => void;
};

export function JobsPage({ onOpenJob, onOpenBatch, onOpenUpload }: Props) {
  const [rows, setRows] = useState<JobListRow[]>([]);
  const [flat, setFlat] = useState<JobSummary[]>([]);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const [nextRows, nextFlat] = await Promise.all([listJobRows(), listJobs()]);
      setRows(nextRows);
      setFlat(nextFlat);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, []);

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

  const stats = useMemo(() => {
    const frequencies = flat.reduce<Record<string, number>>((acc, j) => {
      acc[j.status] = (acc[j.status] ?? 0) + 1;
      return acc;
    }, {});
    return {
      total: flat.length,
      queued: frequencies.queued ?? 0,
      processing: frequencies.processing ?? 0,
      completed: frequencies.completed ?? 0,
      failed: frequencies.failed ?? 0,
    };
  }, [flat]);

  return (
    <section className="page" id="jobs-page">
      <header className="page__header">
        <p className="page__eyebrow">Local</p>
        <h1 className="page__title">Jobs</h1>
        <p className="page__subtitle">
          Track transcription progress, open completed markdown results, and retry failed runs from
          one place.
        </p>
        <div className="page__actions">
          <button type="button" className="btn btn--primary" onClick={onOpenUpload}>
            <IconUploadTray className="size-4" />
            New upload
          </button>
        </div>
      </header>

      <div className="page__body">
        {error && (
          <div className="page-alert" role="alert">
            {error}
          </div>
        )}

        {stats.total > 0 && (
          <div className="jobs-stats" id="jobs-stats">
            <div className="jobs-stats__card jobs-stats__card--total">
              <span className="jobs-stats__value">{stats.total}</span>
              <span className="jobs-stats__label">Total</span>
            </div>
            <div className="jobs-stats__card jobs-stats__card--queued">
              <span className="jobs-stats__value">{stats.queued}</span>
              <span className="jobs-stats__label">Queued</span>
            </div>
            <div className="jobs-stats__card jobs-stats__card--processing">
              <span className="jobs-stats__value">{stats.processing}</span>
              <span className="jobs-stats__label">Processing</span>
            </div>
            <div className="jobs-stats__card jobs-stats__card--completed">
              <span className="jobs-stats__value">{stats.completed}</span>
              <span className="jobs-stats__label">Completed</span>
            </div>
            <div className="jobs-stats__card jobs-stats__card--failed">
              <span className="jobs-stats__value">{stats.failed}</span>
              <span className="jobs-stats__label">Failed</span>
            </div>
          </div>
        )}

        {rows.length === 0 ? (
          <div className="jobs-empty" id="jobs-empty">
            <div className="jobs-empty__icon" aria-hidden="true">
              <IconMusic className="size-7" />
            </div>
            <p className="jobs-empty__title">No jobs yet</p>
            <p className="jobs-empty__text">
              Upload an audio file to start your first transcription.
            </p>
            <button type="button" className="btn btn--primary" onClick={onOpenUpload}>
              <IconUploadTray className="size-4" />
              New upload
            </button>
          </div>
        ) : (
          <div className="jobs-list" id="jobs-list">
            {rows.map((row) =>
              row.kind === "batch" ? (
                <button
                  type="button"
                  key={`batch-${row.batchId}`}
                  id={`job-batch-${row.batchId}`}
                  className="jobs-row jobs-row--batch"
                  onClick={() => onOpenBatch(row.batchId)}
                >
                  <div className="jobs-row__icon" aria-hidden="true">
                    <IconFolder />
                  </div>
                  <div className="jobs-row__main">
                    <p className="jobs-row__title">{row.jobs.length} audios</p>
                    <p className="jobs-row__subtitle">
                      {row.jobs.map((j) => j.originalFilename).join(" · ")}
                    </p>
                  </div>
                  <div className="jobs-row__aside">
                    <div className="jobs-row__meta">
                      <span className="jobs-row__provider">
                        {row.jobs[0]?.providerKey ?? "assemblyai"}
                      </span>
                      <JobStatusBadge status={row.status} />
                      <span className="jobs-row__date">{formatDate(row.createdAt)}</span>
                    </div>
                  </div>
                  <IconChevronRight />
                </button>
              ) : (
                <button
                  type="button"
                  key={`job-${row.job.id}`}
                  id={`job-row-${row.job.id}`}
                  className="jobs-row"
                  onClick={() => onOpenJob(row.job.id)}
                >
                  <div className="jobs-row__icon" aria-hidden="true">
                    <IconMusic />
                  </div>
                  <div className="jobs-row__main">
                    <p className="jobs-row__title">{row.job.originalFilename}</p>
                    {row.job.errorMessage && (
                      <p className="jobs-row__error">{row.job.errorMessage}</p>
                    )}
                  </div>
                  <div className="jobs-row__aside">
                    <div className="jobs-row__meta">
                      <span className="jobs-row__provider">{row.job.providerKey}</span>
                      <JobStatusBadge status={row.job.status} />
                      <span className="jobs-row__date">{formatDate(row.job.createdAt)}</span>
                    </div>
                  </div>
                  <IconChevronRight />
                </button>
              ),
            )}
          </div>
        )}
      </div>
    </section>
  );
}

function formatDate(iso: string) {
  try {
    return new Date(iso).toLocaleString(undefined, {
      month: "short",
      day: "2-digit",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return iso;
  }
}
