import { describe, expect, test } from "bun:test";

import {
  buildOffsets,
  CHILD_ROW_HEIGHT,
  estimateRowHeight,
  indexAtOffset,
  PARENT_ROW_HEIGHT,
  ROW_GAP,
  ROW_HEIGHT,
  totalHeight,
  windowRange,
} from "../src/chat/session-window";

describe("estimateRowHeight", () => {
  test("child rows carry the parent line height even when they have children", () => {
    expect(estimateRowHeight(true, false)).toBe(CHILD_ROW_HEIGHT);
    expect(estimateRowHeight(true, true)).toBe(CHILD_ROW_HEIGHT);
  });

  test("parent rows account for the parent badge row", () => {
    expect(estimateRowHeight(false, true)).toBe(PARENT_ROW_HEIGHT);
    expect(estimateRowHeight(false, false)).toBe(ROW_HEIGHT);
  });

  test("structural heights stay above the CSS minimum", () => {
    for (const height of [ROW_HEIGHT, PARENT_ROW_HEIGHT, CHILD_ROW_HEIGHT]) {
      expect(height).toBeGreaterThanOrEqual(64);
    }
  });
});

describe("buildOffsets", () => {
  test("empty heights produce no offsets", () => {
    expect(buildOffsets([])).toEqual([]);
    expect(totalHeight([])).toBe(0);
  });

  test("offsets accumulate variable heights and the row gap", () => {
    const heights = [ROW_HEIGHT, CHILD_ROW_HEIGHT, ROW_HEIGHT];
    expect(buildOffsets(heights)).toEqual([
      0,
      ROW_HEIGHT + ROW_GAP,
      ROW_HEIGHT + ROW_GAP + CHILD_ROW_HEIGHT + ROW_GAP,
    ]);
    expect(totalHeight(heights)).toBe(
      ROW_HEIGHT + ROW_GAP + CHILD_ROW_HEIGHT + ROW_GAP + ROW_HEIGHT,
    );
  });

  test("a custom gap changes spacing without changing heights", () => {
    expect(buildOffsets([10, 20], 0)).toEqual([0, 10]);
    expect(totalHeight([10, 20], 0)).toBe(30);
  });
});

describe("indexAtOffset", () => {
  const offsets = [0, 82, 190];

  test("clamps before the first row", () => {
    expect(indexAtOffset(offsets, -50)).toBe(0);
    expect(indexAtOffset(offsets, 0)).toBe(0);
  });

  test("finds the row containing or preceding the offset", () => {
    expect(indexAtOffset(offsets, 81)).toBe(0);
    expect(indexAtOffset(offsets, 82)).toBe(1);
    expect(indexAtOffset(offsets, 189)).toBe(1);
    expect(indexAtOffset(offsets, 190)).toBe(2);
  });

  test("clamps past the end and handles an empty list", () => {
    expect(indexAtOffset(offsets, 10_000)).toBe(2);
    expect(indexAtOffset([], 10)).toBe(0);
  });
});

describe("windowRange", () => {
  const offsets = Array.from({ length: 100 }, (_, index) => index * 100);

  test("starts at the top and covers the viewport", () => {
    expect(windowRange(offsets, 0, 300, 0)).toEqual({ start: 0, end: 3 });
  });

  test("moves with the scroll offset", () => {
    expect(windowRange(offsets, 450, 300, 0)).toEqual({ start: 4, end: 8 });
  });

  test("includes overscan above and below", () => {
    expect(windowRange(offsets, 1000, 300, 200)).toEqual({
      start: 8,
      end: 15,
    });
  });

  test("ends at the last row without overflowing", () => {
    expect(windowRange(offsets, 9700, 300, 0)).toEqual({
      start: 97,
      end: 100,
    });
    expect(windowRange(offsets, 100_000, 300, 100)).toEqual({
      start: 99,
      end: 100,
    });
  });

  test("keeps at least one row for a zero-height viewport", () => {
    expect(windowRange(offsets, 0, 0, 0)).toEqual({ start: 0, end: 1 });
  });

  test("empty list has an empty range", () => {
    expect(windowRange([], 0, 300, 200)).toEqual({ start: 0, end: 0 });
  });

  test("variable heights resolve the correct start row", () => {
    const heights = [78, 104, 78, 84];
    const variable = buildOffsets(heights);
    expect(variable).toEqual([0, 82, 190, 272]);
    expect(windowRange(variable, 190, 100, 0)).toEqual({ start: 2, end: 4 });
  });
});
