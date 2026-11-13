import { fetchSnapshot } from "../api";
import { blankDraft, draftFromPreset } from "../drafts";
import { DesktopUnavailableError, errorMessage } from "../tauri";
import { getState, setState } from "../state/store";
import type { EditorTab, Filter } from "../state/store";
import type { Draft, Target } from "../types";
import { applyTheme } from "../ui/theme";
import {
  errorBanner,
  jsonErrorFor,
  selectionFor,
  startupWarningsBanner,
} from "./shared";

export async function loadSnapshot(): Promise<void> {
  const previous = getState();
  const hasData = previous.snapshot !== null;
  setState(
    hasData
      ? { busy: "refresh", error: null, banner: null }
      : { status: "loading", busy: "refresh", error: null, banner: null },
  );
  try {
    const snapshot = await fetchSnapshot();
    applyTheme(snapshot.settings.theme);
    const selection = selectionFor(snapshot, getState().selectedId);
    setState({
      status: "ready",
      busy: null,
      error: null,
      snapshot,
      ...selection,
      tab: "edit",
      selectionToken: getState().selectionToken + 1,
      banner: hasData ? null : startupWarningsBanner(snapshot.startupWarnings),
    });
  } catch (error) {
    if (error instanceof DesktopUnavailableError) {
      setState({
        status: "desktop-required",
        busy: null,
        error: null,
        snapshot: null,
        selectedId: null,
        draft: null,
        baseline: null,
        jsonError: null,
        banner: null,
      });
      return;
    }
    if (hasData) {
      setState({ busy: null, banner: errorBanner(error, "刷新失败") });
      return;
    }
    setState({
      status: "error",
      busy: null,
      error: errorMessage(error),
      snapshot: null,
      selectedId: null,
      draft: null,
      baseline: null,
      jsonError: null,
    });
  }
}

export function selectPreset(id: string): void {
  const state = getState();
  if (state.snapshot === null || state.selectedId === id) {
    return;
  }
  const preset = state.snapshot.presets.find((entry) => entry.id === id);
  if (preset === undefined) {
    return;
  }
  setState({
    selectedId: preset.id,
    draft: draftFromPreset(preset),
    baseline: draftFromPreset(preset),
    jsonError: jsonErrorFor(preset.content),
    tab: "edit",
    banner: null,
    selectionToken: state.selectionToken + 1,
  });
}

export function startNewPreset(): void {
  const state = getState();
  const target: Target =
    state.filter !== "all"
      ? state.filter
      : (state.draft?.target ?? "opencode");
  setState({
    selectedId: null,
    draft: blankDraft(target),
    baseline: blankDraft(target),
    jsonError: null,
    tab: "edit",
    banner: null,
    selectionToken: state.selectionToken + 1,
  });
}

export function updateDraft(patch: Partial<Draft>): void {
  const state = getState();
  if (state.draft === null) {
    return;
  }
  setState({
    draft: { ...state.draft, ...patch },
    jsonError:
      patch.content === undefined
        ? state.jsonError
        : jsonErrorFor(patch.content),
    tab:
      patch.target !== undefined &&
      patch.target !== "omo" &&
      state.tab === "models"
        ? "edit"
        : state.tab,
  });
}

export function bumpSelectionToken(): void {
  const state = getState();
  setState({ selectionToken: state.selectionToken + 1 });
}

export function setFilter(filter: Filter): void {
  setState({ filter });
}

export function setTab(tab: EditorTab): void {
  setState({ tab });
}

export function revertDraft(): void {
  const state = getState();
  if (state.baseline === null) {
    return;
  }
  setState({
    draft: { ...state.baseline },
    jsonError: jsonErrorFor(state.baseline.content),
    banner: null,
    selectionToken: state.selectionToken + 1,
  });
}

export function dismissBanner(): void {
  setState({ banner: null });
}
