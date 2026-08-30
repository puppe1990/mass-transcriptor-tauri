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
import { useLocale } from "../lib/LocaleContext";

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(0)} MB`;
  return `${(n / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

export function SettingsPage() {
  const { t } = useLocale();
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
    const chosen = catalog.find((m) => m.id === whisperModel && m.installed);
    if (defaultProvider === "whisper" && !chosen) {
      setBusy(false);
      setError(t("settings.cannotSave"));
      return;
    }
    try {
      const input: UpdateSettingsInput = {
        workspaceName,
        defaultProvider,
        language,
        whisperModel: chosen?.id ?? settings?.whisperModel ?? whisperModel,
      };
      if (apiKey.trim() !== "") {
        input.assemblyaiApiKey = apiKey.trim();
      }
      const updated = await updateSettings(input);
      setSettings(updated);
      setWhisperModel(updated.whisperModel);
      setApiKey("");
      setMessage(t("settings.saved"));
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
      setMessage(t("settings.downloaded", { name: row.displayName, size: row.sizeLabel }));
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
      const nextCatalog = await listWhisperModels();
      setCatalog(nextCatalog);
      if (whisperModel === id) {
        const fallback = nextCatalog.find((m) => m.installed);
        setWhisperModel(fallback?.id ?? "");
      }
      setMessage(t("settings.removed", { id }));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setDeletingId(null);
    }
  }

  const selectionInstalled = catalog.some((m) => m.id === whisperModel && m.installed);
  const whisperBlocked = defaultProvider === "whisper" && !selectionInstalled;

  return (
    <section className="settings-shell" id="settings-page">
      <div className="settings-shell__intro">
        <p className="settings-shell__eyebrow">{t("settings.eyebrow")}</p>
        <h1>{t("settings.title")}</h1>
        <p className="settings-shell__lede">{t("settings.lede")}</p>

        <div className="settings-shell__note">
          <p className="settings-shell__label">{t("settings.workspace")}</p>
          <strong>{settings?.workspaceName ?? "Local"}</strong>
          <p>{t("settings.workspaceHint")}</p>
          <p>{t("settings.assemblyNote")}</p>
          <p>{t("settings.whisperNote")}</p>
        </div>
      </div>

      <div className="settings-card">
        <form id="settings-form" onSubmit={save}>
          <section className="settings-form__section">
            <p className="settings-shell__label">{t("settings.provider")}</p>
            <label className="settings-form__field">
              <span>{t("settings.defaultProvider")}</span>
              <select
                id="settings-provider"
                value={defaultProvider}
                onChange={(e) => setDefaultProvider(e.target.value)}
                aria-label={t("settings.defaultProvider")}
              >
                <option value="assemblyai">assemblyai</option>
                <option value="whisper">whisper</option>
              </select>
            </label>
            <label className="settings-form__field">
              <span>{t("settings.transcriptionLanguage")}</span>
              <select
                id="settings-language"
                value={language}
                onChange={(e) => setLanguage(e.target.value)}
                aria-label={t("settings.transcriptionLanguage")}
              >
                <option value="auto">{t("settings.langAuto")}</option>
                <option value="pt">{t("settings.langPt")}</option>
                <option value="en">{t("settings.langEn")}</option>
                <option value="es">{t("settings.langEs")}</option>
              </select>
            </label>
          </section>

          <section className="settings-form__section" id="settings-whisper-models">
            <p className="settings-shell__label">{t("settings.models")}</p>
            <p className="settings-models__lede">{t("settings.modelsLede")}</p>
            {whisperBlocked && (
              <p className="settings-models__warn" role="status">
                {t("settings.cannotSave")}
              </p>
            )}
            <ul className="settings-models" aria-label={t("settings.models")}>
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
                    className={[
                      "settings-model",
                      model.installed && whisperModel === model.id && "settings-model--selected",
                      !model.installed && "settings-model--unavailable",
                    ]
                      .filter(Boolean)
                      .join(" ")}
                  >
                    <label className="settings-model__pick">
                      <input
                        type="radio"
                        name="whisper-model"
                        value={model.id}
                        checked={model.installed && whisperModel === model.id}
                        disabled={!model.installed}
                        onChange={() => {
                          if (model.installed) setWhisperModel(model.id);
                        }}
                        aria-label={
                          model.installed
                            ? `Use ${model.displayName}`
                            : t("settings.cannotSelect")
                        }
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
                            {model.installed ? t("settings.onDisk") : t("settings.notDownloaded")}
                          </span>
                        </span>
                        {isDownloading && (
                          <>
                            <span className="settings-model__progress" aria-live="polite">
                              {pct != null
                                ? t("settings.downloadingPct", { pct })
                                : prog
                                  ? t("settings.downloadingBytes", {
                                      bytes: formatBytes(prog.downloadedBytes),
                                    })
                                  : t("settings.downloading")}
                            </span>
                            <span className="settings-model__bar" aria-hidden="true">
                              <i style={{ width: pct != null ? `${pct}%` : "28%" }} />
                            </span>
                          </>
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
                          {deletingId === model.id ? t("settings.removing") : t("settings.remove")}
                        </button>
                      ) : (
                        <button
                          type="button"
                          className="btn btn--secondary"
                          id={`settings-download-${model.id}`}
                          disabled={Boolean(downloadingId)}
                          onClick={() => void handleDownload(model.id)}
                        >
                          {isDownloading ? t("settings.downloading") : t("settings.download")}
                        </button>
                      )}
                    </div>
                  </li>
                );
              })}
            </ul>
            <p className="settings-models__footnote">{t("settings.modelsFootnote")}</p>
          </section>

          <section className="settings-form__section">
            <p className="settings-shell__label">{t("settings.credentials")}</p>
            <div className="settings-form__status-row">
              <span className="settings-shell__label">{t("settings.apiKey")}</span>
              <span
                className={
                  settings?.hasApiKey
                    ? "settings-status settings-status--ok"
                    : "settings-status settings-status--missing"
                }
              >
                {settings?.hasApiKey ? t("settings.configured") : t("settings.notSet")}
              </span>
            </div>
            <label className="settings-form__field">
              <span>{t("settings.apiKeyLabel")}</span>
              <input
                id="settings-api-key"
                type="text"
                autoComplete="off"
                spellCheck={false}
                placeholder={settings?.hasApiKey ? t("settings.keepKey") : t("settings.enterKey")}
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                aria-label={t("settings.apiKey")}
              />
            </label>
          </section>

          <button
            type="submit"
            className="btn btn--primary"
            id="settings-save"
            disabled={busy || whisperBlocked}
          >
            {busy ? t("settings.saving") : t("settings.save")}
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
