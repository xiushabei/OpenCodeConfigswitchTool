import { listModelsCommand } from "../api";
import { errorMessage } from "../tauri";
import type { ModelOption } from "./types";

export type CatalogueStatus = "idle" | "loading" | "ready" | "error";

export interface CatalogueState {
  readonly status: CatalogueStatus;
  readonly models: readonly ModelOption[];
  readonly directory: string;
  readonly error: string | null;
}

type CatalogueListener = (state: CatalogueState) => void;

const listeners = new Set<CatalogueListener>();
let current: CatalogueState = {
  status: "idle",
  models: [],
  directory: "",
  error: null,
};
let pending: Promise<void> | null = null;

export function getCatalogue(): CatalogueState {
  return current;
}

export function subscribeCatalogue(listener: CatalogueListener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function publish(patch: Partial<CatalogueState>): void {
  current = { ...current, ...patch };
  for (const listener of listeners) {
    listener(current);
  }
}

export function loadCatalogue(refresh: boolean): Promise<void> {
  if (pending !== null) {
    return pending;
  }
  // Settle through an explicit resolver so `pending` is set before any listener runs.
  let settle: () => void = () => {};
  const task = new Promise<void>((resolve) => {
    settle = resolve;
  });
  pending = task;
  void (async () => {
    publish({ status: "loading", error: null });
    try {
      const next = await listModelsCommand(refresh);
      publish({
        status: "ready",
        models: next.models,
        directory: next.directory,
        error: null,
      });
    } catch (error) {
      publish({ status: "error", error: errorMessage(error) });
    } finally {
      pending = null;
      settle();
    }
  })();
  return task;
}

/** Resolves once the catalogue is ready, reusing the in-flight request when there is one. */
export function ensureCatalogueReady(): Promise<void> {
  if (pending !== null) {
    return pending;
  }
  if (current.status === "ready") {
    return Promise.resolve();
  }
  return loadCatalogue(current.status === "error");
}

export function ensureCatalogue(): void {
  if (current.status === "idle") {
    void loadCatalogue(false);
  }
}
