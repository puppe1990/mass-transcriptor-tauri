import { useState } from "react";
import { createJobsFromPaths, pickAudioFiles, type CreatedJob } from "../lib/api";
import { IconAlert, IconCheck, IconUploadTray } from "../components/icons";

type Props = {
  onCreated: (jobs: CreatedJob[]) => void;
  onOpenJobs: () => void;
};

export function UploadPage({ onCreated, onOpenJobs }: Props) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [last, setLast] = useState<CreatedJob[]>([]);

  async function handlePick() {
    setError(null);
    try {
      const paths = await pickAudioFiles();
      if (paths.length === 0) return;
      setBusy(true);
      const jobs = await createJobsFromPaths(paths);
      setLast(jobs);
      onCreated(jobs);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="page" id="upload-page">
      <header className="page__header">
        <p className="page__eyebrow">Local</p>
        <h1 className="page__title">Upload Audio or Video</h1>
        <p className="page__subtitle">
          Drop audio or short video files here. Files are transcribed with AssemblyAI using your
          local API key.
        </p>
        <div className="page__actions">
          <a
            href="#jobs"
            onClick={(e) => {
              e.preventDefault();
              onOpenJobs();
            }}
          >
            View jobs
          </a>
        </div>
      </header>

      <div className="page__body">
        {last.length > 0 && (
          <div id="upload-success" className="upload-success">
            <div className="upload-success__icon" aria-hidden="true">
              <IconCheck />
            </div>
            <div className="upload-success__content">
              <strong>
                {last.length === 1
                  ? "1 job queued for transcription."
                  : `${last.length} jobs queued for transcription.`}
              </strong>
              <p className="upload-success__links">
                {last[0]?.batchId != null ? (
                  <a
                    href={`#batch-${last[0].batchId}`}
                    onClick={(e) => {
                      e.preventDefault();
                      onCreated(last);
                    }}
                  >
                    Open upload group
                  </a>
                ) : (
                  last.map((j, i) => (
                    <span key={j.id}>
                      {i > 0 ? ", " : null}
                      Job #{j.id}
                    </span>
                  ))
                )}
              </p>
            </div>
          </div>
        )}

        <div className="upload-form">
          <div
            className={[
              "upload-dropzone",
              busy ? "upload-dropzone--active" : "",
              error ? "upload-dropzone--error" : "",
            ]
              .filter(Boolean)
              .join(" ")}
            id="upload-dropzone"
          >
            <div className="upload-dropzone__icon" aria-hidden="true">
              <IconUploadTray />
            </div>
            <p className="upload-dropzone__title">Drag and drop audio or video files here</p>
            <p className="upload-dropzone__hint">
              MP3, WAV, OGG, M4A, FLAC or MP4, MOV, WebM, MKV · up to 20 files
            </p>
            <button
              type="button"
              className="btn btn--primary upload-dropzone__browse"
              id="pick-files-btn"
              disabled={busy}
              onClick={() => void handlePick()}
            >
              {busy ? "Creating jobs…" : "Browse files"}
            </button>
          </div>

          {error && (
            <div id="upload-error" className="upload-error upload-error--banner" role="alert">
              <IconAlert className="size-5 shrink-0" />
              <div>
                <strong>Cannot upload this file</strong>
                <p>{error}</p>
              </div>
            </div>
          )}
        </div>
      </div>
    </section>
  );
}
