/** DOM window mechanics for the virtualized chat session list. */

import type { SessionEntry, SessionRowView } from "./session-rows";
import {
  applyMeasurements,
  buildOffsets,
  estimateRowHeight,
  indexAtOffset,
  totalHeight,
  windowRange,
} from "./session-window";

const OVERSCAN_PX = 600;
const FALLBACK_VIEWPORT = 480;

export interface SessionWindowSource {
  element: HTMLElement;
  inner: HTMLElement;
  rows: SessionRowView;
  entries(): readonly SessionEntry[];
  activeIndex(): number;
}

export interface SessionWindowView {
  offsets(): readonly number[];
  scrollTop(): number;
  viewportHeight(): number;
  reset(entries: readonly SessionEntry[]): void;
  clear(): void;
  syncWidth(): void;
  update(): void;
  measure(): void;
  reveal(index: number): void;
  relayout(): void;
  attach(): void;
}

export function createSessionWindowView(
  source: SessionWindowSource,
): SessionWindowView {
  const { element, inner, rows } = source;
  let heights: number[] = [];
  let offsets: number[] = [];
  let measured: boolean[] = [];
  let frame = 0;
  let pendingRelayout = false;
  let lastWidth = -1;

  function viewportHeight(): number {
    return element.clientHeight > 0 ? element.clientHeight : FALLBACK_VIEWPORT;
  }

  function measure(): void {
    if (source.entries().length === 0) {
      return;
    }
    const readings = rows.readUnmeasured(measured);
    if (readings.length === 0) {
      return;
    }
    const result = applyMeasurements(
      { heights, offsets, measured, scrollTop: element.scrollTop },
      readings,
    );
    if (!result.changed) {
      return;
    }
    element.scrollTop = result.scrollTop;
    inner.style.height = `${totalHeight(heights)}px`;
    rows.positionAll();
  }

  function update(): void {
    const entries = source.entries();
    if (entries.length === 0) {
      return;
    }
    const activeIndex = source.activeIndex();
    const range = windowRange(
      offsets,
      element.scrollTop,
      viewportHeight(),
      OVERSCAN_PX,
    );
    const keep = new Set<number>();
    for (let index = range.start; index < range.end; index += 1) {
      keep.add(index);
    }
    if (activeIndex >= 0 && activeIndex < entries.length) {
      keep.add(activeIndex);
    }
    rows.prune(keep);
    for (let index = range.start; index < range.end; index += 1) {
      if (!rows.ensure(index)) {
        rows.applyState(index);
        rows.position(index);
      }
    }
    if (activeIndex >= 0 && activeIndex < entries.length) {
      if (!rows.ensure(activeIndex)) {
        rows.applyState(activeIndex);
        rows.position(activeIndex);
      }
    }
    rows.syncTabIndex();
    measure();
  }

  function schedule(kind: "update" | "relayout"): void {
    if (source.entries().length === 0) {
      return;
    }
    if (kind === "relayout") {
      pendingRelayout = true;
    }
    if (frame !== 0) {
      return;
    }
    frame = window.requestAnimationFrame(() => {
      frame = 0;
      const relayoutPending = pendingRelayout;
      pendingRelayout = false;
      if (relayoutPending) {
        relayout();
      } else {
        update();
      }
    });
  }

  function relayout(): void {
    const entries = source.entries();
    if (entries.length === 0) {
      return;
    }
    const anchor = indexAtOffset(offsets, element.scrollTop);
    const anchorDelta = (offsets[anchor] ?? 0) - element.scrollTop;
    heights = entries.map((entry) =>
      estimateRowHeight(entry.session.parentId !== null, entry.hasChildren),
    );
    offsets = buildOffsets(heights);
    measured = new Array<boolean>(entries.length).fill(false);
    inner.style.height = `${totalHeight(heights)}px`;
    // Keep mounted rows: their content reflows at the new width in place, so
    // the same elements are re-measured instead of being destroyed/rebuilt.
    element.scrollTop = Math.max(0, (offsets[anchor] ?? 0) - anchorDelta);
    update();
    rows.positionAll();
  }

  return {
    offsets() {
      return offsets;
    },
    scrollTop() {
      return element.scrollTop;
    },
    viewportHeight,
    reset(entries) {
      heights = entries.map((entry) =>
        estimateRowHeight(entry.session.parentId !== null, entry.hasChildren),
      );
      offsets = buildOffsets(heights);
      measured = new Array<boolean>(entries.length).fill(false);
      inner.style.height = `${totalHeight(heights)}px`;
    },
    clear() {
      heights = [];
      offsets = [];
      measured = [];
      pendingRelayout = false;
      if (frame !== 0) {
        window.cancelAnimationFrame(frame);
        frame = 0;
      }
    },
    syncWidth() {
      lastWidth = element.clientWidth;
    },
    update,
    measure,
    reveal(index) {
      const top = offsets[index] ?? 0;
      const bottom = top + (heights[index] ?? 0);
      const height = viewportHeight();
      const scrollTop = element.scrollTop;
      if (top < scrollTop) {
        element.scrollTop = top;
      } else if (bottom > scrollTop + height) {
        element.scrollTop = bottom - height;
      }
      update();
      rows.syncTabIndex();
      rows.focus(index);
    },
    relayout,
    attach() {
      const resizeObserver = new ResizeObserver(() => {
        if (source.entries().length === 0) {
          return;
        }
        const width = element.clientWidth;
        if (width !== lastWidth) {
          lastWidth = width;
          schedule("relayout");
          return;
        }
        schedule("update");
      });
      resizeObserver.observe(element);
      element.addEventListener("scroll", () => schedule("update"), {
        passive: true,
      });
    },
  };
}
