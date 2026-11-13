import type { ChatDiscovery } from "./types";

/** Join discovery and per-command warnings into one visible note. */
export function chatNotice(
  discovery: ChatDiscovery | null,
  warnings: readonly string[],
): string | null {
  const lines: string[] = [];
  if (discovery !== null) {
    if (discovery.searchedAllDrives) {
      lines.push("默认位置未命中，已扫描全部固定和可移动驱动器。");
    }
    lines.push(...discovery.warnings);
  }
  lines.push(...warnings);
  const clean = lines.filter((line) => line.trim().length > 0);
  return clean.length > 0 ? clean.join("\n") : null;
}
