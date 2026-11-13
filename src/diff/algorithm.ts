import type {
  Counters,
  DiffLine,
  Span,
} from "./model";
import { MAX_DP_CELLS } from "./model";
import { findAnchors } from "./anchor";

export interface DiffContext {
  left: readonly string[];
  right: readonly string[];
  out: DiffLine[];
  counters: Counters;
}

export function pushLine(ctx: DiffContext, line: DiffLine): void {
  ctx.out.push(line);
  if (line.kind === "add") {
    ctx.counters.added += 1;
  } else if (line.kind === "remove") {
    ctx.counters.removed += 1;
  }
}

function addLine(
  ctx: DiffContext,
  index: number,
  oldLine: number | null,
): void {
  pushLine(ctx, {
    kind: "add",
    text: ctx.right[index] ?? "",
    oldLine,
    newLine: index + 1,
  });
}

function removeLine(
  ctx: DiffContext,
  index: number,
  newLine: number | null,
): void {
  pushLine(ctx, {
    kind: "remove",
    text: ctx.left[index] ?? "",
    oldLine: index + 1,
    newLine,
  });
}

function sameLine(
  ctx: DiffContext,
  leftIndex: number,
  rightIndex: number,
): void {
  pushLine(ctx, {
    kind: "same",
    text: ctx.left[leftIndex] ?? "",
    oldLine: leftIndex + 1,
    newLine: rightIndex + 1,
  });
}

function diffWithLcs(ctx: DiffContext, span: Span): void {
  const leftSize = span.leftEnd - span.leftStart;
  const rightSize = span.rightEnd - span.rightStart;
  const width = rightSize + 1;
  const table = new Uint32Array((leftSize + 1) * width);
  for (let row = leftSize - 1; row >= 0; row -= 1) {
    for (let column = rightSize - 1; column >= 0; column -= 1) {
      const cell = row * width + column;
      if (ctx.left[span.leftStart + row] === ctx.right[span.rightStart + column]) {
        table[cell] = (table[(row + 1) * width + column + 1] ?? 0) + 1;
      } else {
        const down = table[(row + 1) * width + column] ?? 0;
        const next = table[row * width + column + 1] ?? 0;
        table[cell] = down >= next ? down : next;
      }
    }
  }
  let row = 0;
  let column = 0;
  while (row < leftSize && column < rightSize) {
    const leftIndex = span.leftStart + row;
    const rightIndex = span.rightStart + column;
    if (ctx.left[leftIndex] === ctx.right[rightIndex]) {
      sameLine(ctx, leftIndex, rightIndex);
      row += 1;
      column += 1;
      continue;
    }
    const down = table[(row + 1) * width + column] ?? 0;
    const next = table[row * width + column + 1] ?? 0;
    if (down >= next) {
      removeLine(ctx, leftIndex, null);
      row += 1;
    } else {
      addLine(ctx, rightIndex, null);
      column += 1;
    }
  }
  while (row < leftSize) {
    removeLine(ctx, span.leftStart + row, null);
    row += 1;
  }
  while (column < rightSize) {
    addLine(ctx, span.rightStart + column, null);
    column += 1;
  }
}

function diffMiddle(ctx: DiffContext, span: Span): void {
  const leftSize = span.leftEnd - span.leftStart;
  const rightSize = span.rightEnd - span.rightStart;
  if (leftSize === 0) {
    for (let index = span.rightStart; index < span.rightEnd; index += 1) {
      addLine(ctx, index, null);
    }
    return;
  }
  if (rightSize === 0) {
    for (let index = span.leftStart; index < span.leftEnd; index += 1) {
      removeLine(ctx, index, null);
    }
    return;
  }
  if (leftSize * rightSize <= MAX_DP_CELLS) {
    diffWithLcs(ctx, span);
    return;
  }
  const anchors = findAnchors(ctx.left, ctx.right, span);
  if (anchors.length === 0) {
    for (let index = span.leftStart; index < span.leftEnd; index += 1) {
      removeLine(ctx, index, null);
    }
    for (let index = span.rightStart; index < span.rightEnd; index += 1) {
      addLine(ctx, index, null);
    }
    return;
  }
  let leftCursor = span.leftStart;
  let rightCursor = span.rightStart;
  for (const anchor of anchors) {
    diffRange(ctx, {
      leftStart: leftCursor,
      leftEnd: anchor.leftIndex,
      rightStart: rightCursor,
      rightEnd: anchor.rightIndex,
    });
    sameLine(ctx, anchor.leftIndex, anchor.rightIndex);
    leftCursor = anchor.leftIndex + 1;
    rightCursor = anchor.rightIndex + 1;
  }
  diffRange(ctx, {
    leftStart: leftCursor,
    leftEnd: span.leftEnd,
    rightStart: rightCursor,
    rightEnd: span.rightEnd,
  });
}

export function diffRange(ctx: DiffContext, span: Span): void {
  let leftCursor = span.leftStart;
  let rightCursor = span.rightStart;
  while (
    leftCursor < span.leftEnd &&
    rightCursor < span.rightEnd &&
    ctx.left[leftCursor] === ctx.right[rightCursor]
  ) {
    sameLine(ctx, leftCursor, rightCursor);
    leftCursor += 1;
    rightCursor += 1;
  }
  let leftTail = span.leftEnd;
  let rightTail = span.rightEnd;
  while (
    leftTail > leftCursor &&
    rightTail > rightCursor &&
    ctx.left[leftTail - 1] === ctx.right[rightTail - 1]
  ) {
    leftTail -= 1;
    rightTail -= 1;
  }
  diffMiddle(ctx, {
    leftStart: leftCursor,
    leftEnd: leftTail,
    rightStart: rightCursor,
    rightEnd: rightTail,
  });
  const suffix = span.leftEnd - leftTail;
  for (let index = 0; index < suffix; index += 1) {
    sameLine(ctx, leftTail + index, rightTail + index);
  }
}
