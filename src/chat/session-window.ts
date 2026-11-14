/** Pure variable-height window policy for the chat session list. */

export const ROW_GAP = 4;
export const ROW_HEIGHT = 78;
export const PARENT_ROW_HEIGHT = 84;
export const CHILD_ROW_HEIGHT = 104;

export interface WindowRange {
  /** First rendered index, inclusive. */
  start: number;
  /** Last rendered index, exclusive. */
  end: number;
}

/** Structural row heights match the rendered CSS at every supported width. */
export function estimateRowHeight(
  isChild: boolean,
  hasChildren: boolean,
): number {
  if (isChild) {
    return CHILD_ROW_HEIGHT;
  }
  return hasChildren ? PARENT_ROW_HEIGHT : ROW_HEIGHT;
}

export function buildOffsets(
  heights: readonly number[],
  gap: number = ROW_GAP,
): number[] {
  const offsets = new Array<number>(heights.length);
  let cursor = 0;
  for (let index = 0; index < heights.length; index += 1) {
    offsets[index] = cursor;
    cursor += (heights[index] ?? 0) + gap;
  }
  return offsets;
}

export function totalHeight(
  heights: readonly number[],
  gap: number = ROW_GAP,
): number {
  if (heights.length === 0) {
    return 0;
  }
  let total = 0;
  for (const height of heights) {
    total += height;
  }
  return total + gap * (heights.length - 1);
}

/** Largest index whose offset is at or before `position`, or 0. */
export function indexAtOffset(
  offsets: readonly number[],
  position: number,
): number {
  if (offsets.length === 0) {
    return 0;
  }
  let low = 0;
  let high = offsets.length - 1;
  let found = 0;
  while (low <= high) {
    const middle = (low + high) >> 1;
    if ((offsets[middle] ?? 0) <= position) {
      found = middle;
      low = middle + 1;
    } else {
      high = middle - 1;
    }
  }
  return found;
}

function firstAtOrAfter(offsets: readonly number[], position: number): number {
  let low = 0;
  let high = offsets.length;
  while (low < high) {
    const middle = (low + high) >> 1;
    if ((offsets[middle] ?? 0) < position) {
      low = middle + 1;
    } else {
      high = middle;
    }
  }
  return low;
}

export function windowRange(
  offsets: readonly number[],
  scrollTop: number,
  viewport: number,
  overscan: number,
): WindowRange {
  const count = offsets.length;
  if (count === 0) {
    return { start: 0, end: 0 };
  }
  const safeScroll = Number.isFinite(scrollTop) ? Math.max(0, scrollTop) : 0;
  const safeViewport = Number.isFinite(viewport) ? Math.max(0, viewport) : 0;
  const safeOverscan = Number.isFinite(overscan) ? Math.max(0, overscan) : 0;
  const first = Math.max(0, safeScroll - safeOverscan);
  const last = safeScroll + safeViewport + safeOverscan;

  const start = indexAtOffset(offsets, first);
  let end = firstAtOrAfter(offsets, last);
  if (end <= start) {
    end = Math.min(count, start + 1);
  }
  return { start, end: Math.min(count, end) };
}

export interface HeightReading {
  index: number;
  height: number;
}

export interface MeasurementState {
  heights: number[];
  offsets: number[];
  measured: boolean[];
  scrollTop: number;
}

export interface MeasurementResult {
  changed: boolean;
  scrollTop: number;
}

/**
 * Folds fresh row measurements into the window model. Rows above the scroll
 * position shift the scroll offset so the visible rows stay anchored.
 */
export function applyMeasurements(
  state: MeasurementState,
  readings: readonly HeightReading[],
): MeasurementResult {
  const { heights, offsets, measured } = state;
  let scrollTop = state.scrollTop;
  let changed = false;
  for (const { index, height } of readings) {
    measured[index] = true;
    const previous = heights[index] ?? height;
    if (height <= 0 || Math.abs(height - previous) <= 0.5) {
      continue;
    }
    const delta = height - previous;
    if ((offsets[index] ?? 0) < scrollTop) {
      scrollTop += delta;
    }
    heights[index] = height;
    for (let next = index + 1; next < heights.length; next += 1) {
      offsets[next] = (offsets[next] ?? 0) + delta;
    }
    changed = true;
  }
  return { changed, scrollTop };
}

export interface FocusNavigation {
  key: string;
  activeIndex: number;
  count: number;
  offsets: readonly number[];
  scrollTop: number;
  viewport: number;
}

/** Target row for a navigation key, or null when the key is not handled. */
export function focusTargetForKey(navigation: FocusNavigation): number | null {
  switch (navigation.key) {
    case "ArrowDown":
      return navigation.activeIndex + 1;
    case "ArrowUp":
      return navigation.activeIndex - 1;
    case "Home":
      return 0;
    case "End":
      return navigation.count - 1;
    case "PageDown":
      return indexAtOffset(
        navigation.offsets,
        navigation.scrollTop + navigation.viewport * 0.9,
      );
    case "PageUp":
      return indexAtOffset(
        navigation.offsets,
        Math.max(0, navigation.scrollTop - navigation.viewport * 0.9),
      );
    default:
      return null;
  }
}
