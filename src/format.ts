import type { Target } from "./types";

export const TARGET_LABELS: Record<Target, string> = {
  opencode: "OpenCode",
  omo: "OMO",
};

export function targetLabel(target: Target): string {
  return TARGET_LABELS[target];
}
