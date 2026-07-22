import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export type AppSettings = {
  workspaceName: string;
  defaultProvider: string;
  assemblyaiApiKey: string | null;
  hasApiKey: boolean;
  language: string;
};

export type JobSummary = {
  id: number;
  status: string;
  providerKey: string;
  batchId: number | null;
  uploadId: number;
  originalFilename: string;
  errorMessage: string | null;
  createdAt: string;
  retryable: boolean;
};

export type JobDetail = {
  id: number;
  status: string;
  providerKey: string;
  batchId: number | null;
  uploadId: number;
  originalFilename: string;
  errorMessage: string | null;
  markdownPath: string | null;
  transcriptText: string | null;
  createdAt: string;
  startedAt: string | null;
  completedAt: string | null;
  retryable: boolean;
};

export type BatchDetail = {
  id: number;
  createdAt: string;
  jobs: JobDetail[];
};

export type CreatedJob = {
  id: number;
  uploadId: number;
  batchId: number | null;
  status: string;
  originalFilename: string;
};

export type UpdateSettingsInput = {
  workspaceName: string;
  defaultProvider: string;
  assemblyaiApiKey?: string | null;
  language: string;
};

export async function getSettings(): Promise<AppSettings> {
  return invoke("get_settings");
}

export async function updateSettings(input: UpdateSettingsInput): Promise<AppSettings> {
  return invoke("update_settings", { input });
}

export async function listJobs(): Promise<JobSummary[]> {
  return invoke("list_jobs");
}

/** Grouped jobs list (batch rows + singles). */
export type JobListRow =
  | { kind: "single"; job: JobSummary }
  | {
      kind: "batch";
      batchId: number;
      jobs: JobSummary[];
      createdAt: string;
      status: string;
    };

export async function listJobRows(): Promise<JobListRow[]> {
  return invoke("list_job_rows");
}

export async function getJob(jobId: number): Promise<JobDetail | null> {
  return invoke("get_job", { jobId });
}

export async function getBatch(batchId: number): Promise<BatchDetail | null> {
  return invoke("get_batch", { batchId });
}

export async function retryJob(jobId: number): Promise<JobDetail> {
  return invoke("retry_job", { jobId });
}

export async function getTranscriptMarkdown(jobId: number): Promise<string> {
  return invoke("get_transcript_markdown", { jobId });
}

export type BatchTranscriptFile = {
  jobId: number;
  filename: string;
  markdown: string;
};

export async function listBatchTranscripts(batchId: number): Promise<BatchTranscriptFile[]> {
  return invoke("list_batch_transcripts", { batchId });
}

/** Base64-encoded ZIP of all completed transcripts in the batch. */
export async function downloadBatchTranscriptsZip(batchId: number): Promise<string> {
  return invoke("download_batch_transcripts_zip", { batchId });
}

export async function createJobsFromPaths(paths: string[]): Promise<CreatedJob[]> {
  return invoke("create_jobs_from_paths", { paths });
}

export async function pickAudioFiles(): Promise<string[]> {
  const selected = await open({
    multiple: true,
    filters: [
      {
        name: "Audio / Video",
        extensions: [
          "wav",
          "mp3",
          "ogg",
          "opus",
          "m4a",
          "flac",
          "webm",
          "aac",
          "mp4",
          "mov",
          "mkv",
        ],
      },
    ],
  });
  if (!selected) return [];
  return Array.isArray(selected) ? selected : [selected];
}

export async function appInfo(): Promise<{
  dbPath: string;
  storageRoot: string;
  auth: boolean;
  entry: string;
}> {
  return invoke("app_info");
}
