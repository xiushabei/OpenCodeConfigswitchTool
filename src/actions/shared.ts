import { checkJsonObject, draftFromPreset } from "../drafts";
import { errorMessage } from "../tauri";
import type { Banner, Filter, UiState } from "../state/store";
import type { Preset, Snapshot } from "../types";

export function errorBanner(error: unknown, prefix: string): Banner {
  return {
    tone: "error",
    message: `${prefix}：${errorMessage(error)}`,
    detail: null,
  };
}

export function infoBanner(message: string): Banner {
  return { tone: "info", message, detail: null };
}

export function startupWarningsBanner(
  warnings: readonly string[],
): Banner | null {
  if (warnings.length === 0) {
    return null;
  }
  return {
    tone: "info",
    message: `启动配置提示（${warnings.length} 条）`,
    detail: warnings.join("\n"),
  };
}

export function jsonErrorFor(content: string): string | null {
  const check = checkJsonObject(content);
  return check.ok ? null : check.message;
}

export function pickPreset(
  snapshot: Snapshot,
  preferredId: string | null,
): Preset | null {
  if (preferredId !== null) {
    const found = snapshot.presets.find((preset) => preset.id === preferredId);
    if (found !== undefined) {
      return found;
    }
  }
  return snapshot.presets[0] ?? null;
}

export type Selection = Pick<
  UiState,
  "selectedId" | "draft" | "baseline" | "jsonError"
>;

export function selectionFor(
  snapshot: Snapshot,
  preferredId: string | null,
): Selection {
  const preset = pickPreset(snapshot, preferredId);
  if (preset === null) {
    return {
      selectedId: null,
      draft: null,
      baseline: null,
      jsonError: null,
    };
  }
  return {
    selectedId: preset.id,
    draft: draftFromPreset(preset),
    baseline: draftFromPreset(preset),
    jsonError: jsonErrorFor(preset.content),
  };
}

export function findSaved(
  snapshot: Snapshot,
  preset: Preset,
  beforeIds: ReadonlySet<string>,
): Preset | null {
  if (preset.id.length > 0) {
    return snapshot.presets.find((entry) => entry.id === preset.id) ?? null;
  }
  return snapshot.presets.find((entry) => !beforeIds.has(entry.id)) ?? null;
}

export function filterFor(state: UiState, preset: Preset): Filter {
  if (state.filter === "all" || state.filter === preset.target) {
    return state.filter;
  }
  return "all";
}
