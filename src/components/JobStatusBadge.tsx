import { useLocale } from "../lib/LocaleContext";

const STATUS_KEYS = ["queued", "processing", "completed", "failed", "cancelled"] as const;

export function JobStatusBadge({ status }: { status: string }) {
  const { t } = useLocale();
  const key = STATUS_KEYS.includes(status as (typeof STATUS_KEYS)[number])
    ? `status.${status}`
    : "status.unknown";
  return <span className={`status status-${status}`}>{t(key)}</span>;
}
