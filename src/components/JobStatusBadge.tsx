const LABELS: Record<string, string> = {
  queued: "Queued",
  processing: "Processing",
  completed: "Completed",
  failed: "Failed",
};

export function JobStatusBadge({ status }: { status: string }) {
  return <span className={`status status-${status}`}>{LABELS[status] ?? "Unknown"}</span>;
}
