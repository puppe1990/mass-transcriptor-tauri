import { useCallback, useEffect, useState } from "react";
import { getBatch, type BatchDetail } from "../lib/api";
import { JobStatusBadge } from "../components/JobStatusBadge";
import { IconChevronLeft, IconChevronRight, IconMusic } from "../components/icons";

type Props = {
  batchId: number;
  onBack: () => void;
  onOpenJob: (id: number) => void;
};

export function BatchPage({ batchId, onBack, onOpenJob }: Props) {
  const [batch, setBatch] = useState<BatchDetail | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const detail = await getBatch(batchId);
      setBatch(detail);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [batchId]);

  useEffect(() => {
    void refresh();
    const id = window.setInterval(() => void refresh(), 2000);
    return () => window.clearInterval(id);
  }, [refresh]);

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
        <h1 className="page__title">Batch #{batch.id}</h1>
        <p className="page__subtitle">{batch.jobs.length} jobs in this upload group</p>
      </header>

      <div className="page__body">
        <div className="jobs-list" id="batch-jobs-list">
          {batch.jobs.map((job) => (
            <button
              type="button"
              key={job.id}
              className="jobs-row"
              onClick={() => onOpenJob(job.id)}
            >
              <div className="jobs-row__icon" aria-hidden="true">
                <IconMusic />
              </div>
              <div className="jobs-row__main">
                <p className="jobs-row__title">{job.originalFilename}</p>
                {job.errorMessage && <p className="jobs-row__error">{job.errorMessage}</p>}
              </div>
              <div className="jobs-row__aside">
                <div className="jobs-row__meta">
                  <span className="jobs-row__provider">{job.providerKey}</span>
                  <JobStatusBadge status={job.status} />
                </div>
              </div>
              <IconChevronRight />
            </button>
          ))}
        </div>
      </div>
    </section>
  );
}
