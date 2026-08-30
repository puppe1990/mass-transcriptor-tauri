import { useEffect, useState, type FormEvent } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  deleteWhisperModel,
  downloadWhisperModel,
  getSettings,
  listWhisperModels,
  updateSettings,
  type AppSettings,
  type UpdateSettingsInput,
  type WhisperModelProgress,
  type WhisperModelStatus,
} from "../lib/api";

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(0)} MB`;
  return `${(n / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

export function SettingsPage() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [workspaceName, setWorkspaceName] = useState("");
  const [defaultProvider, setDefaultProvider] = useState("assemblyai");
  const [apiKey, setApiKey] = useState("");
  const [language, setLanguage] = useState("auto");
  const [whisperModel, setWhisperModel] = useState("base");
  const [catalog, setCatalog] = useState<WhisperModelStatus[]>([]);
  const [downloadingId, setDownloadingId] = useState<string | null>(null);
  const [progress, setProgress] = useState<WhisperModelProgress | null>(null);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function refreshCatalog() {
    const models = await listWhisperModels();
    setCatalog(models);
  }

  useEffect(() => {
    void getSettings()
      .then((s) => {
        setSettings(s);
        setWorkspaceName(s.workspaceName);
        setDefaultProvider(s.defaultProvider);
        setLanguage(s.language);
        setWhisperModel(s.whisperModel);
        setApiKey("");
      })
      .catch((e) => setError(e instanceof Error ? e.message : String(e)));
    void refreshCatalog().catch((e) => setError(e instanceof Error ? e.message : String(e)));
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<WhisperModelProgress>("whisper-model-progress", (ev) => {
      setProgress(ev.payload);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  async function save(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    setMessage(null);
    setError(null);
    try {
      const input: UpdateSettingsInput = {
        workspaceName,
        defaultProvider,
        language,
        whisperModel,
      };
      if (apiKey.trim() !== "") {
        input.assemblyaiApiKey = apiKey.trim();
      }
      const updated = await updateSettings(input);
      setSettings(updated);
      setWhisperModel(updated.whisperModel);
      setApiKey("");
      setMessage("Settings saved.");
      await refreshCatalog();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleDownload(id: string) {
    setDownloadingId(id);
    setProgress({ id, downloadedBytes: 0, totalBytes: null });
    setError(null);
    setMessage(null);
    try {
      const row = await downloadWhisperModel(id);
      await refreshCatalog();
      setMessage(`Downloaded ${row.displayName} (${row.sizeLabel}). Select it and save to use it on Whisper jobs.`);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setDownloadingId(null);
      setProgress(null);
    }
  }

  async function handleDelete(id: string) {
    setDeletingId(id);
    setError(null);
    setMessage(null);
    try {
      await deleteWhisperModel(id);
      await refreshCatalog();
      setMessage(`Removed ${id} from this machine.`);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setDeletingId(null);
    }
  }

  const selectedRow = catalog.find((m) => m.id === whisperModel);
  const selectedMissing = Boolean(selectedRow && !selectedRow.installed);

  return (
    <section className="settings-shell" id="settings-page">
      <div className="settings-shell__intro">
        <p className="settings-shell__eyebrow">Workspace controls</p>
        <h1>Provider Settings</h1>
        <p className="settings-shell__lede">
          Choose which engine runs each transcript and keep external credentials scoped to this
          machine only.
        </p>

        <div className="settings-shell__note">
          <p className="settings-shell__label">Workspace</p>
          <strong>{settings?.workspaceName ?? "Local"}</strong>
          <p>Local desktop app · no multi-user login</p>
          <p>AssemblyAI uses the API key stored in local SQLite settings.</p>
          <p>Whisper is local and does not use that key. Models stay on disk until you delete them.</p>
        </div>
      </div>

      <div className="settings-card">
        <form id="settings-form" onSubmit={save}>
          <section className="settings-form__section">
            <p className="settings-shell__label">Workspace</p>
            <label className="settings-form__field">
              <span>Workspace</span>
              <input
                id="settings-workspace"
                type="text"
                value={workspaceName}
                onChange={(e) => setWorkspaceName(e.target.value)}
                placeholder="Your workspace"
                aria-label="Workspace name"
                required
              />
            </label>
          </section>

          <section className="settings-form__section">
            <p className="settings-shell__label">Provider</p>
            <label className="settings-form__field">
              <span>Default provider</span>
              <select
                id="settings-provider"
                value={defaultProvider}
                onChange={(e) => setDefaultProvider(e.target.value)}
                aria-label="Default provider"
              >
                <option value="assemblyai">assemblyai</option>
                <option value="whisper">whisper</option>
              </select>
            </label>
            <label className="settings-form__field">
              <span>Transcription language</span>
              <select
                id="settings-language"
                value={language}
                onChange={(e) => setLanguage(e.target.value)}
                aria-label="Transcription language"
              >
                <option value="auto">Auto detect</option>
                <option value="pt">Portuguese</option>
                <option value="en">English</option>
                <option value="es">Spanish</option>
              </select>
            </label>
          </section>

          <section className="settings-form__section" id="settings-whisper-models">
            <p className="settings-shell__label">Local Whisper models</p>
            <p className="settings-models__lede">
              Download only the sizes you want. Nothing is fetched until you click Download. Whisper
              jobs fail until the selected model is on disk.
            </p>
            {selectedMissing && defaultProvider === "whisper" && (
              <p className="settings-models__warn" role="status">
                Selected model `{whisperModel}` is not downloaded yet.
              </p>
            )}
            <ul className="settings-models" aria-label="Whisper models">
              {catalog.map((model) => {
                const isDownloading = downloadingId === model.id;
                const prog = isDownloading ? progress : null;
                const pct =
                  prog && prog.totalBytes && prog.totalBytes > 0
                    ? Math.min(100, Math.round((prog.downloadedBytes / prog.totalBytes) * 100))
                    : null;
                return (
                  <li
                    key={model.id}
                    className={
                      whisperModel === model.id
                        ? "settings-model settings-model--selected"
                        : "settings-model"
                    }
                  >
                    <label className="settings-model__pick">
                      <input
                        type="radio"
                        name="whisper-model"
                        value={model.id}
                        checked={whisperModel === model.id}
                        onChange={() => setWhisperModel(model.id)}
                        aria-label={`Use ${model.displayName}`}
                      />
                      <span className="settings-model__copy">
                        <strong>{model.displayName}</strong>
                        <span className="settings-model__hint">{model.qualityHint}</span>
                        <span className="settings-model__meta">
                          <span>{model.sizeLabel}</span>
                          <span
                            className={
                              model.installed
                                ? "settings-status settings-status--ok"
                                : "settings-status settings-status--missing"
                            }
                          >
                            {model.installed ? "On disk" : "Not downloaded"}
                          </span>
                        </span>
                        {isDownloading && (
                          <span className="settings-model__progress" aria-live="polite">
                            {pct != null
                              ? `Downloading ${pct}%`
                              : prog
                                ? `Downloading ${formatBytes(prog.downloadedBytes)}`
                                : "Downloading…"}
                          </span>
                        )}
                      </span>
                    </label>
                    <div className="settings-model__actions">
                      {model.installed ? (
                        <button
                          type="button"
                          className="btn btn--ghost"
                          id={`settings-delete-${model.id}`}
                          disabled={Boolean(downloadingId) || deletingId === model.id}
                          onClick={() => void handleDelete(model.id)}
                        >
                          {deletingId === model.id ? "Removing…" : "Remove"}
                        </button>
                      ) : (
                        <button
                          type="button"
                          className="btn btn--secondary"
                          id={`settings-download-${model.id}`}
                          disabled={Boolean(downloadingId)}
                          onClick={() => void handleDownload(model.id)}
                        >
                          {isDownloading ? "Downloading…" : "Download"}
                        </button>
                      )}
                    </div>
                  </li>
                );
              })}
            </ul>
            <p className="settings-models__footnote">
              Files come from Hugging Face (<code>ggerganov/whisper.cpp</code>). Handy models such as
              Nemotron, Parakeet, Voxtral, Qwen3-ASR, Fun-ASR, and Cohere Transcribe need
              transcribe.cpp, which this app does not run.
            </p>
          </section>

          <section className="settings-form__section">
            <p className="settings-shell__label">Credentials</p>
            <div className="settings-form__status-row">
              <span className="settings-shell__label">AssemblyAI API key</span>
              <span
                className={
                  settings?.hasApiKey
                    ? "settings-status settings-status--ok"
                    : "settings-status settings-status--missing"
                }
              >
                {settings?.hasApiKey ? "Configured" : "Not set"}
              </span>
            </div>
            <label className="settings-form__field">
              <span>API key</span>
              <input
                id="settings-api-key"
                type="text"
                autoComplete="off"
                spellCheck={false}
                placeholder={
                  settings?.hasApiKey ? "•••••••• (leave blank to keep)" : "Enter API key"
                }
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                aria-label="AssemblyAI API key"
              />
            </label>
          </section>

          <button type="submit" className="btn btn--primary" id="settings-save" disabled={busy}>
            {busy ? "Saving…" : "Save settings"}
          </button>
        </form>

        {message && (
          <p className="settings-form__footer" id="settings-success" style={{ marginTop: 16 }}>
            {message}
          </p>
        )}
        {error && (
          <p className="page-alert" id="settings-error" role="alert" style={{ marginTop: 16 }}>
            {error}
          </p>
        )}
      </div>
    </section>
  );
}
