import { useEffect, useState, type FormEvent } from "react";
import { getSettings, updateSettings, type AppSettings, type UpdateSettingsInput } from "../lib/api";

export function SettingsPage() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [workspaceName, setWorkspaceName] = useState("");
  const [defaultProvider, setDefaultProvider] = useState("assemblyai");
  const [apiKey, setApiKey] = useState("");
  const [language, setLanguage] = useState("auto");
  const [whisperModel, setWhisperModel] = useState("base");
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

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
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

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
          <p>Whisper is local and does not use that key.</p>
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
            {defaultProvider === "whisper" && (
              <>
                <label className="settings-form__field">
                  <span>Whisper model</span>
                  <select
                    id="settings-whisper-model"
                    value={whisperModel}
                    onChange={(e) => setWhisperModel(e.target.value)}
                    aria-label="Whisper model"
                  >
                    <option value="tiny">tiny (~75 MB)</option>
                    <option value="base">base (~142 MB)</option>
                    <option value="small">small (~466 MB)</option>
                  </select>
                </label>
                <p className="settings-shell__lede">
                  The first Whisper job for a model size downloads a ggml file into app data.
                  ffmpeg must be installed and available as the <code>ffmpeg</code> command.
                </p>
              </>
            )}
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
