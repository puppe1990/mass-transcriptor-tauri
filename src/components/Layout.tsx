import { useEffect, useState, type ReactNode } from "react";
import { getSettings } from "../lib/api";
import { useLocale } from "../lib/LocaleContext";
import { IconJobs, IconSettings, IconUploads } from "./icons";
import { LanguageSelect } from "./LanguageSelect";
import { ThemeToggle } from "./ThemeToggle";

export type Route = "upload" | "jobs" | "job" | "batch" | "settings";

type Props = {
  route: Route;
  onNavigate: (route: Route, id?: number) => void;
  children: ReactNode;
};

export function Layout({ route, onNavigate, children }: Props) {
  const { t } = useLocale();
  const [workspace, setWorkspace] = useState("Local");

  useEffect(() => {
    void getSettings()
      .then((s) => setWorkspace(s.workspaceName || "Local"))
      .catch(() => setWorkspace("Local"));
  }, [route]);

  const jobsActive = route === "jobs" || route === "job" || route === "batch";

  return (
    <div className="app-shell">
      <nav className="app-sidebar" aria-label={t("sidebar.ariaLabel")}>
        <div className="app-sidebar__brand">
          <div className="app-sidebar__logo" aria-hidden="true">
            <span className="mark">
              <i />
              <i />
              <i />
              <i />
              <i />
            </span>
          </div>
          <div className="app-sidebar__brand-text">
            <p className="app-sidebar__product">Mass Transcriptor</p>
            <strong>{workspace}</strong>
          </div>
        </div>

        <div className="app-sidebar__links">
          <a
            href="#upload"
            id="nav-upload"
            className={route === "upload" ? "active" : undefined}
            onClick={(e) => {
              e.preventDefault();
              onNavigate("upload");
            }}
          >
            <IconUploads />
            {t("sidebar.uploads")}
          </a>
          <a
            href="#jobs"
            id="nav-jobs"
            className={jobsActive ? "active" : undefined}
            onClick={(e) => {
              e.preventDefault();
              onNavigate("jobs");
            }}
          >
            <IconJobs />
            {t("sidebar.jobs")}
          </a>
          <a
            href="#settings"
            id="nav-settings"
            className={route === "settings" ? "active" : undefined}
            onClick={(e) => {
              e.preventDefault();
              onNavigate("settings");
            }}
          >
            <IconSettings />
            {t("sidebar.settings")}
          </a>
        </div>

        <div className="app-sidebar__footer">
          <ThemeToggle />
          <LanguageSelect />
          <p className="app-sidebar__meta">{t("sidebar.onThisMachine")}</p>
        </div>
      </nav>

      <main className="app-content" id="main-workspace">
        {children}
      </main>
    </div>
  );
}
