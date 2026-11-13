import type { DiffLine, DiffResult } from "./model";
import {
  MAX_RENDER_LINES,
  MAX_SOURCE_LINES,
  splitLines,
} from "./model";
import type { DiffContext } from "./algorithm";
import { diffRange, pushLine } from "./algorithm";

export function diffLines(leftText: string, rightText: string): DiffResult {
  if (leftText === rightText) {
    return {
      lines: [],
      added: 0,
      removed: 0,
      identical: true,
      truncated: false,
    };
  }
  const left = splitLines(leftText);
  const right = splitLines(rightText);
  const lines: DiffLine[] = [];
  const ctx: DiffContext = {
    left,
    right,
    out: lines,
    counters: { added: 0, removed: 0 },
  };
  const oversized =
    left.length > MAX_SOURCE_LINES || right.length > MAX_SOURCE_LINES;

  if (oversized) {
    const cappedLeft = left.slice(0, MAX_SOURCE_LINES);
    const cappedRight = right.slice(0, MAX_SOURCE_LINES);
    for (const [index, line] of cappedLeft.entries()) {
      pushLine(ctx, {
        kind: "remove",
        text: line,
        oldLine: index + 1,
        newLine: null,
      });
    }
    for (const [index, line] of cappedRight.entries()) {
      pushLine(ctx, {
        kind: "add",
        text: line,
        oldLine: null,
        newLine: index + 1,
      });
    }
  } else {
    diffRange(ctx, {
      leftStart: 0,
      leftEnd: left.length,
      rightStart: 0,
      rightEnd: right.length,
    });
  }

  const requiresSlice = lines.length > MAX_RENDER_LINES;
  return {
    lines: requiresSlice ? lines.slice(0, MAX_RENDER_LINES) : lines,
    added: ctx.counters.added,
    removed: ctx.counters.removed,
    identical: ctx.counters.added === 0 && ctx.counters.removed === 0,
    truncated: oversized || requiresSlice,
  };
}
