import { draftsEqual } from "../drafts";
import type { Preset, Snapshot, Target, TargetState } from "../types";
import type { UiState } from "./store";

export function isDirty(state: UiState): boolean {
  return !draftsEqual(state.draft, state.baseline);
}

export function selectedPreset(state: UiState): Preset | null {
  if (state.snapshot === null || state.selectedId === null) {
    return null;
  }
  return (
    state.snapshot.presets.find((preset) => preset.id === state.selectedId) ??
    null
  );
}

export function targetStateFor(
  snapshot: Snapshot | null,
  target: Target,
): TargetState | null {
  if (snapshot === null) {
    return null;
  }
  return snapshot.targets.find((entry) => entry.target === target) ?? null;
}

export function targetPathFor(state: UiState, target: Target): string {
  const entry = targetStateFor(state.snapshot, target);
  if (entry !== null) {
    return entry.path;
  }
  if (state.snapshot !== null) {
    return state.snapshot.settings.targets[target];
  }
  return "";
}

export function activePresetIds(state: UiState): ReadonlySet<string> {
  const ids = new Set<string>();
  for (const entry of state.snapshot?.targets ?? []) {
    for (const id of entry.activePresetIds) {
      ids.add(id);
    }
  }
  return ids;
}

export function filteredPresets(state: UiState): Preset[] {
  const presets = state.snapshot?.presets ?? [];
  if (state.filter === "all") {
    return presets;
  }
  return presets.filter((preset) => preset.target === state.filter);
}

export function presetCountFor(state: UiState, target: Target): number {
  return (state.snapshot?.presets ?? []).filter(
    (preset) => preset.target === target,
  ).length;
}

export function targetHasError(state: UiState, target: Target): boolean {
  const entry = targetStateFor(state.snapshot, target);
  return entry !== null && entry.error !== null;
}
