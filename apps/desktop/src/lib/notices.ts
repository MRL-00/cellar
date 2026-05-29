import type { DatabaseNotice, NoticeSeverity } from "@cellar/ipc";

export type NoticeTone = "danger" | "warning" | "info" | "muted";

export const NOTICE_SEVERITIES: NoticeSeverity[] = [
  "panic",
  "fatal",
  "error",
  "warning",
  "notice",
  "info",
  "log",
  "debug",
  "unknown",
];

export function countNoticeSeverities(
  notices: DatabaseNotice[],
): Record<NoticeSeverity, number> {
  const counts = Object.fromEntries(
    NOTICE_SEVERITIES.map((s) => [s, 0]),
  ) as Record<NoticeSeverity, number>;
  for (const notice of notices) {
    counts[notice.severity] += 1;
  }
  return counts;
}

export function toneForSeverity(severity: NoticeSeverity): NoticeTone {
  switch (severity) {
    case "panic":
    case "fatal":
    case "error":
      return "danger";
    case "warning":
      return "warning";
    case "notice":
    case "info":
      return "info";
    case "log":
    case "debug":
    case "unknown":
      return "muted";
  }
}

export function formatNoticeTime(timestamp: string): string {
  const date = new Date(timestamp);
  if (Number.isNaN(date.getTime())) return timestamp;
  return date.toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}
