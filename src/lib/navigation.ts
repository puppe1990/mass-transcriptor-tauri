import type { CreatedJob } from "./api";

export type AppRoute = "upload" | "jobs" | "job" | "batch" | "settings";

export type NavigationTarget =
  | { route: "upload" | "jobs" | "settings" }
  | { route: "job"; id: number }
  | { route: "batch"; id: number };

/** Where to go after create_jobs_from_paths succeeds. */
export function targetAfterUpload(jobs: CreatedJob[]): NavigationTarget {
  if (jobs.length === 0) {
    return { route: "jobs" };
  }
  if (jobs.length === 1) {
    return { route: "job", id: jobs[0].id };
  }
  const batchId = jobs.find((j) => j.batchId != null)?.batchId;
  if (batchId != null) {
    return { route: "batch", id: batchId };
  }
  return { route: "jobs" };
}

/** Open batch from jobs list row (guards missing/undefined id from bad payloads). */
export function targetOpenBatch(batchId: number | null | undefined): NavigationTarget {
  if (batchId == null || !Number.isFinite(batchId)) {
    return { route: "jobs" };
  }
  return { route: "batch", id: batchId };
}
