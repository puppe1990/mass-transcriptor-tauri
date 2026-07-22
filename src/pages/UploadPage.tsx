import { useCallback, useEffect, useState, type DragEvent } from "react";
import { createJobsFromPaths, pickAudioFiles, type CreatedJob } from "../lib/api";
import { IconAlert, IconCheck, IconUploadTray } from "../components/icons";

type Props = {
  onCreated: (jobs: CreatedJob[]) => void;
  onOpenJobs: () => void;
};

const AUDIO_VIDEO_EXT =
  /\.(wav|mp3|ogg|opus|m4a|flac|webm|aac|wma|mpga|oga|mp4|mov|mkv)$/i;

function filterMediaPaths(paths: string[]): string[] {
  return paths.filter((p) => AUDIO_VIDEO_EXT.test(p));
}

export function UploadPage({ onCreated, onOpenJobs }: Props) {
  const [busy, setBusy] = useState(false);
  const [dragging, setDragging] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [last, setLast] = useState<CreatedJob[]>([]);

  const uploadPaths = useCallback(
    async (paths: string[]) => {
      const media = filterMediaPaths(paths);
      if (media.length === 0) {
        setError(
          "This file type is not supported. Use common audio formats or MP4, MOV, WebM, and MKV video.",
        );
        return;
      }
      setError(null);
      setBusy(true);
      try {
        const jobs = await createJobsFromPaths(media);
        setLast(jobs);
        onCreated(jobs);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setBusy(false);
      }
    },
    [onCreated],
  );

  // Native Tauri drag-and-drop (gives real filesystem paths — required on desktop).
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    void (async () => {
      try {
        const { getCurrentWebview } = await import("@tauri-apps/api/webview");
        if (cancelled) return;
        unlisten = await getCurrentWebview().onDragDropEvent((event) => {
          const { type } = event.payload;
          if (type === "enter" || type === "over") {
            setDragging(true);
          } else if (type === "leave") {
            setDragging(false);
          } else if (type === "drop") {
            setDragging(false);
            const paths = event.payload.paths ?? [];
            if (paths.length > 0) {
              void uploadPaths(paths);
            }
          }
        });
      } catch {
        // Not running inside Tauri (e.g. browser-only vite) — HTML handlers below cover it.
      }
    })();

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [uploadPaths]);

  async function handlePick() {
    setError(null);
    try {
      const paths = await pickAudioFiles();
      if (paths.length === 0) return;
      await uploadPaths(paths);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  // HTML5 fallback (browser / when webview still delivers FileList).
  function onDragOver(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    setDragging(true);
  }

  function onDragLeave(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    setDragging(false);
  }

  async function onHtmlDrop(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    setDragging(false);

    const files = Array.from(e.dataTransfer?.files ?? []);
    if (files.length === 0) return;

    // In Tauri, HTML File often has no path; prefer native drop event above.
    // When path is available (webkitRelativePath / path on some webviews), use it.
    const paths: string[] = [];
    for (const file of files) {
      const withPath = file as File & { path?: string };
      if (withPath.path) {
        paths.push(withPath.path);
      }
    }
    if (paths.length > 0) {
      await uploadPaths(paths);
      return;
    }

    // Last resort: write temp files via backend
    try {
      setBusy(true);
      setError(null);
      const { invoke } = await import("@tauri-apps/api/core");
      const tempPaths: string[] = [];
      for (const file of files) {
        const buf = new Uint8Array(await file.arrayBuffer());
        const path = await invoke<string>("write_temp_upload", {
          filename: file.name,
          bytes: Array.from(buf),
        });
        tempPaths.push(path);
      }
      await uploadPaths(tempPaths);
    } catch (err) {
      setError(
        err instanceof Error
          ? err.message
          : "Drop failed. Use Browse files, or drop files while the app window is focused.",
      );
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
              (busy || dragging) && "upload-dropzone--active",
              error && "upload-dropzone--error",
            ]
              .filter(Boolean)
              .join(" ")}
            id="upload-dropzone"
            onDragEnter={onDragOver}
            onDragOver={onDragOver}
            onDragLeave={onDragLeave}
            onDrop={(e) => void onHtmlDrop(e)}
          >
            <div className="upload-dropzone__icon" aria-hidden="true">
              <IconUploadTray />
            </div>
            <p className="upload-dropzone__title">
              {dragging
                ? "Drop files to queue transcription"
                : "Drag and drop audio or video files here"}
            </p>
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
