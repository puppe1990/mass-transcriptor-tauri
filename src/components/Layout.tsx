import { useEffect, useState, type ReactNode } from "react";
import { getSettings } from "../lib/api";
import { IconJobs, IconSettings, IconUploads } from "./icons";
import { ThemeToggle } from "./ThemeToggle";

export type Route = "upload" | "jobs" | "job" | "batch" | "settings";

type Props = {
  route: Route;
  onNavigate: (route: Route, id?: number) => void;
  children: ReactNode;
};

export function Layout({ route, onNavigate, children }: Props) {
  const [workspace, setWorkspace] = useState("Local");

  useEffect(() => {
    void getSettings()
      .then((s) => setWorkspace(s.workspaceName || "Local"))
      .catch(() => setWorkspace("Local"));
  }, [route]);

  const jobsActive = route === "jobs" || route === "job" || route === "batch";

  return (
    <div className="app-shell">
      <nav className="app-sidebar" aria-label="Workspace sidebar">
        <div className="app-sidebar__brand">
          <div className="app-sidebar__logo" aria-hidden="true">
            M
          </div>
          <div className="app-sidebar__brand-text">
            <p className="app-sidebar__eyebrow">Mass Transcriptor</p>
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
            Uploads
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
            Jobs
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
            Settings
          </a>
        </div>

        <div className="app-sidebar__footer">
          <ThemeToggle />
          <p className="app-sidebar__field" style={{ margin: 0 }}>
            <span>Local desktop · no login</span>
          </p>
        </div>
      </nav>

      <main className="app-content" id="main-workspace">
        {children}
      </main>
    </div>
  );
}
