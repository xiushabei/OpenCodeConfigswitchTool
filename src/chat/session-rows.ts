/** Row collection and row-level DOM for the windowed chat session list. */

import { createSessionRow } from "./session-row";
import type { ChatSession } from "./types";

export interface SessionEntry {
  session: ChatSession;
  hasChildren: boolean;
  parentTitle: string | null;
}

export interface RowReading {
  index: number;
  height: number;
}

/** Presentation values the controller owns at any moment. */
export interface SessionRowState {
  selectedId: string | null;
  disabled: boolean;
  activeIndex: number;
}

export interface SessionRowSource {
  host: HTMLElement;
  entryAt(index: number): SessionEntry | undefined;
  state(): SessionRowState;
  offsetOf(index: number): number;
  onSelect(session: ChatSession, index: number): void;
}

export interface SessionRowView {
  /** Creates the row when absent; returns whether a row was created. */
  ensure(index: number): boolean;
  has(index: number): boolean;
  indices(): Iterable<number>;
  applyState(index: number): void;
  position(index: number): void;
  positionAll(): void;
  syncTabIndex(): void;
  prune(keep: ReadonlySet<number>): void;
  clear(): void;
  focus(index: number): void;
  /** Reads heights of rows not yet measured, in render order. */
  readUnmeasured(measured: readonly boolean[]): RowReading[];
}

export function buildSessionEntries(
  sessions: readonly ChatSession[],
): SessionEntry[] {
  const byId = new Map<string, ChatSession>();
  const parents = new Set<string>();
  for (const session of sessions) {
    byId.set(session.id, session);
    if (session.parentId !== null) {
      parents.add(session.parentId);
    }
  }
  return sessions.map((session) => ({
    session,
    hasChildren: parents.has(session.id),
    parentTitle:
      session.parentId === null
        ? null
        : byId.get(session.parentId)?.title ?? null,
  }));
}

export function createSessionRowView(
  source: SessionRowSource,
): SessionRowView {
  const rendered = new Map<number, HTMLButtonElement>();

  function position(index: number): void {
    const row = rendered.get(index);
    if (row !== undefined) {
      row.style.transform = `translateY(${source.offsetOf(index)}px)`;
    }
  }

  function insertOrdered(index: number, row: HTMLButtonElement): void {
    let next: HTMLButtonElement | null = null;
    let nextIndex = Number.POSITIVE_INFINITY;
    for (const [candidate, node] of rendered) {
      if (candidate > index && candidate < nextIndex) {
        nextIndex = candidate;
        next = node;
      }
    }
    source.host.insertBefore(row, next);
  }

  function applyState(index: number): void {
    const row = rendered.get(index);
    if (row === undefined) {
      return;
    }
    const entry = source.entryAt(index);
    const state = source.state();
    row.setAttribute(
      "aria-pressed",
      String(entry !== undefined && entry.session.id === state.selectedId),
    );
    row.tabIndex = index === state.activeIndex ? 0 : -1;
    row.disabled = state.disabled;
  }

  function create(index: number): void {
    const entry = source.entryAt(index);
    if (entry === undefined) {
      throw new Error(`missing session at ${index}`);
    }
    const row = createSessionRow(entry.session, {
      hasChildren: entry.hasChildren,
      parentTitle: entry.parentTitle,
    });
    row.addEventListener("click", () => {
      if (row.disabled) {
        return;
      }
      source.onSelect(entry.session, index);
    });
    rendered.set(index, row);
    insertOrdered(index, row);
    applyState(index);
    position(index);
  }

  return {
    ensure(index) {
      if (rendered.has(index)) {
        return false;
      }
      create(index);
      return true;
    },
    has(index) {
      return rendered.has(index);
    },
    indices() {
      return rendered.keys();
    },
    applyState,
    position,
    positionAll() {
      for (const index of rendered.keys()) {
        position(index);
      }
    },
    syncTabIndex() {
      const activeIndex = source.state().activeIndex;
      for (const [index, row] of rendered) {
        row.tabIndex = index === activeIndex ? 0 : -1;
      }
    },
    prune(keep) {
      for (const [index, row] of rendered) {
        if (!keep.has(index)) {
          row.remove();
          rendered.delete(index);
        }
      }
    },
    clear() {
      for (const row of rendered.values()) {
        row.remove();
      }
      rendered.clear();
    },
    focus(index) {
      rendered.get(index)?.focus({ preventScroll: true });
    },
    readUnmeasured(measured) {
      const pending: { index: number; row: HTMLButtonElement }[] = [];
      for (const [index, row] of rendered) {
        if (measured[index] !== true) {
          pending.push({ index, row });
        }
      }
      return pending.map(({ index, row }) => ({
        index,
        height: row.getBoundingClientRect().height,
      }));
    },
  };
}
