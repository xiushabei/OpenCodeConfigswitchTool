import { describe, expect, test } from "bun:test";

import { createSessionWindowView } from "../src/chat/session-window-view";
import type {
  SessionEntry,
  SessionRowView,
} from "../src/chat/session-rows";
import type { ChatSession } from "../src/chat/types";

interface Frame {
  id: number;
  callback: FrameRequestCallback;
}

interface Harness {
  view: ReturnType<typeof createSessionWindowView>;
  stats: { clear: number; positionAll: number };
  mounted: Set<number>;
  notifyResize(): void;
  flushFrame(): boolean;
  element: { clientWidth: number; scrollTop: number };
}

function entry(index: number): SessionEntry {
  const session: ChatSession = {
    id: `s${index}`,
    title: `会话 ${index}`,
    directory: "D:\\qa",
    parentId: null,
    created: index,
    updated: index,
    sizeBytes: index,
    messageCount: 1,
  };
  return { session, hasChildren: false, parentTitle: null };
}

function setup(): Harness {
  const frames: Frame[] = [];
  let frameId = 1;
  let resizeCallback: ResizeObserverCallback | null = null;

  const globals = globalThis as unknown as {
    window: {
      requestAnimationFrame(callback: FrameRequestCallback): number;
      cancelAnimationFrame(id: number): void;
    };
    ResizeObserver: new (callback: ResizeObserverCallback) => unknown;
  };
  globals.window = {
    requestAnimationFrame(callback) {
      const id = frameId;
      frameId += 1;
      frames.push({ id, callback });
      return id;
    },
    cancelAnimationFrame(id) {
      const index = frames.findIndex((frame) => frame.id === id);
      if (index >= 0) {
        frames.splice(index, 1);
      }
    },
  };
  globals.ResizeObserver = class {
    constructor(callback: ResizeObserverCallback) {
      resizeCallback = callback;
    }
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  };

  const entries = Array.from({ length: 200 }, (_, index) => entry(index));
  const mounted = new Set<number>();
  const stats = { clear: 0, positionAll: 0 };
  const rows: SessionRowView = {
    ensure(index) {
      if (mounted.has(index)) {
        return false;
      }
      mounted.add(index);
      return true;
    },
    has(index) {
      return mounted.has(index);
    },
    indices() {
      return mounted.keys();
    },
    applyState() {},
    position() {},
    positionAll() {
      stats.positionAll += 1;
    },
    syncTabIndex() {},
    prune(keep) {
      for (const index of [...mounted]) {
        if (!keep.has(index)) {
          mounted.delete(index);
        }
      }
    },
    clear() {
      stats.clear += 1;
      mounted.clear();
    },
    focus() {},
    readUnmeasured() {
      return [];
    },
  };

  const element = {
    clientHeight: 400,
    clientWidth: 800,
    scrollTop: 0,
    addEventListener() {},
  } as unknown as HTMLElement;
  const inner = { style: { height: "" } } as unknown as HTMLElement;
  const view = createSessionWindowView({
    element,
    inner,
    rows,
    entries: () => entries,
    activeIndex: () => 0,
  });
  view.reset(entries);
  view.attach();
  view.update();

  if (resizeCallback === null) {
    throw new Error("resize observer was not attached");
  }
  const notify = resizeCallback as ResizeObserverCallback;

  return {
    view,
    stats,
    mounted,
    element,
    notifyResize: () => notify([], {} as ResizeObserver),
    flushFrame: () => {
      const frame = frames.shift();
      if (frame === undefined) {
        return false;
      }
      frame.callback(performance.now());
      return true;
    },
  };
}

describe("session window resize scheduling", () => {
  test("rapid width changes coalesce into one relayout per frame", () => {
    const harness = setup();
    const before = [...harness.mounted];
    expect(harness.stats.positionAll).toBe(0);

    harness.element.clientWidth = 720;
    harness.notifyResize();
    harness.element.clientWidth = 640;
    harness.notifyResize();
    harness.element.clientWidth = 560;
    harness.notifyResize();
    expect(harness.stats.positionAll).toBe(0);

    expect(harness.flushFrame()).toBe(true);
    expect(harness.stats.positionAll).toBe(1);
    expect(harness.flushFrame()).toBe(false);

    expect(harness.stats.clear).toBe(0);
    expect([...harness.mounted]).toEqual(before);
  });

  test("same-width notifications update without rebuilding row geometry", () => {
    const harness = setup();
    harness.element.clientWidth = 720;
    harness.notifyResize();
    harness.flushFrame();
    expect(harness.stats.positionAll).toBe(1);

    harness.element.clientHeight = 300;
    harness.notifyResize();
    harness.flushFrame();
    expect(harness.stats.positionAll).toBe(1);
    expect(harness.stats.clear).toBe(0);
  });

  test("a width relayout keeps the scroll anchor", () => {
    const harness = setup();
    const offsets = harness.view.offsets();
    const anchorTop = offsets[80] ?? 0;
    harness.element.scrollTop = anchorTop + 12;

    harness.element.clientWidth = 700;
    harness.notifyResize();
    harness.flushFrame();

    expect(harness.element.scrollTop).toBe(anchorTop + 12);
    expect(harness.stats.clear).toBe(0);
    expect(harness.mounted.size).toBeGreaterThan(0);
  });
});
