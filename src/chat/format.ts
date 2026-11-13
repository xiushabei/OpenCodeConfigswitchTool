import type { ChatSort } from "./types";

const TIME_FORMAT = new Intl.DateTimeFormat("zh-CN", {
  year: "numeric",
  month: "2-digit",
  day: "2-digit",
  hour: "2-digit",
  minute: "2-digit",
  hour12: false,
});

const SIZE_UNITS = ["B", "KB", "MB", "GB", "TB"] as const;

const SORT_LABELS: Record<ChatSort, string> = {
  size: "内容体积",
  updated: "更新时间",
  created: "创建时间",
};

/** Render a byte total; session totals are content bytes, database totals are file lengths. */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "0 B";
  }
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < SIZE_UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const text =
    unit === 0 ? String(Math.round(value)) : value.toFixed(value >= 100 ? 0 : 1);
  return `${text} ${SIZE_UNITS[unit] ?? "TB"}`;
}

export function formatTime(value: number): string {
  if (!Number.isFinite(value) || value <= 0) {
    return "—";
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return "—";
  }
  return TIME_FORMAT.format(date);
}

export function sortLabel(sort: ChatSort): string {
  return SORT_LABELS[sort];
}

export function sessionTitle(title: string): string {
  return title.trim().length > 0 ? title : "未命名会话";
}
