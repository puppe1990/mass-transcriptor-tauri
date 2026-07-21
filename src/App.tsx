import { useState } from "react";
import { Layout, type Route } from "./components/Layout";
import { UploadPage } from "./pages/UploadPage";
import { JobsPage } from "./pages/JobsPage";
import { JobPage } from "./pages/JobPage";
import { BatchPage } from "./pages/BatchPage";
import { SettingsPage } from "./pages/SettingsPage";
import type { CreatedJob } from "./lib/api";
import "./styles.css";

/** Client-side navigation only — no auth routes or redirects. */
export default function App() {
  const [route, setRoute] = useState<Route>("upload");
  const [jobId, setJobId] = useState<number | null>(null);
  const [batchId, setBatchId] = useState<number | null>(null);

  function navigate(next: Route, id?: number) {
    setRoute(next);
    if (next === "job" && id != null) setJobId(id);
    if (next === "batch" && id != null) setBatchId(id);
    if (next === "jobs" || next === "upload" || next === "settings") {
      setJobId(null);
      setBatchId(null);
    }
  }

  function onCreated(jobs: CreatedJob[]) {
    if (jobs.length === 1) {
      navigate("job", jobs[0].id);
    } else if (jobs[0]?.batchId != null) {
      navigate("batch", jobs[0].batchId);
    } else {
      navigate("jobs");
    }
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
      {route === "settings" && <SettingsPage />}
    </Layout>
  );
}
