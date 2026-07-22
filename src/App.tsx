import { useState } from "react";
import { Layout, type Route } from "./components/Layout";
import { UploadPage } from "./pages/UploadPage";
import { JobsPage } from "./pages/JobsPage";
import { JobPage } from "./pages/JobPage";
import { BatchPage } from "./pages/BatchPage";
import { SettingsPage } from "./pages/SettingsPage";
import type { CreatedJob } from "./lib/api";
import { targetAfterUpload, targetOpenBatch, type NavigationTarget } from "./lib/navigation";
import "./styles.css";

/** Client-side navigation only — no auth routes or redirects. */
export default function App() {
  const [route, setRoute] = useState<Route>("upload");
  const [jobId, setJobId] = useState<number | null>(null);
  const [batchId, setBatchId] = useState<number | null>(null);

  function applyTarget(target: NavigationTarget) {
    setRoute(target.route);
    if (target.route === "job") {
      setJobId(target.id);
      setBatchId(null);
    } else if (target.route === "batch") {
      setBatchId(target.id);
      setJobId(null);
    } else {
      setJobId(null);
      setBatchId(null);
    }
  }

  function navigate(next: Route, id?: number) {
    if (next === "job") {
      if (id != null) {
        applyTarget({ route: "job", id });
      } else {
        applyTarget({ route: "jobs" });
      }
      return;
    }
    if (next === "batch") {
      applyTarget(targetOpenBatch(id));
      return;
    }
    // upload | jobs | settings — no id
    applyTarget({ route: next });
  }

  function onCreated(jobs: CreatedJob[]) {
    applyTarget(targetAfterUpload(jobs));
  }

  return (
    <Layout route={route} onNavigate={navigate}>
      {route === "upload" && (
        <UploadPage onCreated={onCreated} onOpenJobs={() => navigate("jobs")} />
      )}
      {route === "jobs" && (
        <JobsPage
          onOpenJob={(id) => navigate("job", id)}
          onOpenBatch={(id) => navigate("batch", id)}
          onOpenUpload={() => navigate("upload")}
        />
      )}
      {route === "job" && jobId != null && (
        <JobPage
          jobId={jobId}
          onBack={() => navigate("jobs")}
          onOpenBatch={(id) => navigate("batch", id)}
        />
      )}
      {route === "batch" && batchId != null && (
        <BatchPage
          batchId={batchId}
          onBack={() => navigate("jobs")}
          onOpenJob={(id) => navigate("job", id)}
        />
      )}
      {route === "batch" && batchId == null && (
        <section className="page">
          <div className="page-alert" role="alert">
            Batch not found. Open it again from Jobs.
          </div>
          <button type="button" className="btn btn--primary" onClick={() => navigate("jobs")}>
            View jobs
          </button>
        </section>
      )}
      {route === "settings" && <SettingsPage />}
    </Layout>
  );
}
