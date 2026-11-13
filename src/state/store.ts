import type { Draft, Snapshot, Target } from "../types";

export type AppStatus = "loading" | "ready" | "desktop-required" | "error";
export type EditorTab = "edit" | "models" | "diff";
export type Filter = "all" | Target;
export type BusyAction =
  | "refresh"
  | "save"
  | "apply"
  | "delete"
  | "duplicate"
  | "export"
  | "import"
  | "restore"
  | "settings";
export type BannerTone = "success" | "error" | "info";

export interface Banner {
  tone: BannerTone;
  message: string;
  detail: string | null;
}

export interface UiState {
  status: AppStatus;
  error: string | null;
  snapshot: Snapshot | null;
  selectedId: string | null;
  draft: Draft | null;
  baseline: Draft | null;
  filter: Filter;
  tab: EditorTab;
  busy: BusyAction | null;
  banner: Banner | null;
  selectionToken: number;
  jsonError: string | null;
}

export type Listener = (state: UiState) => void;

const initialState: UiState = {
  status: "loading",
  error: null,
  snapshot: null,
  selectedId: null,
  draft: null,
  baseline: null,
  filter: "all",
  tab: "edit",
  busy: null,
  banner: null,
  selectionToken: 0,
  jsonError: null,
};

let current: UiState = initialState;
const listeners = new Set<Listener>();

export function getState(): UiState {
  return current;
}

export function setState(patch: Partial<UiState>): void {
  current = { ...current, ...patch };
  for (const listener of listeners) {
    listener(current);
  }
}

export function subscribe(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}
