import type { Anchor, Span } from "./model";

function countUnique(
  lines: readonly string[],
  start: number,
  end: number,
): Map<string, number> {
  const counts = new Map<string, number>();
  for (let index = start; index < end; index += 1) {
    const line = lines[index] ?? "";
    counts.set(line, (counts.get(line) ?? 0) + 1);
  }
  return counts;
}

function collectIndices(
  lines: readonly string[],
  start: number,
  end: number,
): Map<string, number> {
  const indices = new Map<string, number>();
  for (let index = start; index < end; index += 1) {
    const line = lines[index] ?? "";
    if (!indices.has(line)) {
      indices.set(line, index);
    }
  }
  return indices;
}

function longestIncreasing(anchors: readonly Anchor[]): Anchor[] {
  const predecessors = new Array<number>(anchors.length).fill(-1);
  const tails: number[] = [];
  for (let index = 0; index < anchors.length; index += 1) {
    const value = anchors[index]?.rightIndex ?? 0;
    let low = 0;
    let high = tails.length;
    while (low < high) {
      const middle = (low + high) >> 1;
      const tailIndex = tails[middle] ?? 0;
      const tailValue = anchors[tailIndex]?.rightIndex ?? 0;
      if (tailValue < value) {
        low = middle + 1;
      } else {
        high = middle;
      }
    }
    predecessors[index] = low > 0 ? (tails[low - 1] ?? -1) : -1;
    tails[low] = index;
  }
  const result: Anchor[] = [];
  let cursor = tails.length > 0 ? (tails[tails.length - 1] ?? -1) : -1;
  while (cursor >= 0) {
    const anchor = anchors[cursor];
    if (anchor !== undefined) {
      result.push(anchor);
    }
    cursor = predecessors[cursor] ?? -1;
  }
  result.reverse();
  return result;
}

/** Unique common lines (patience anchors) in increasing right-side order. */
export function findAnchors(
  left: readonly string[],
  right: readonly string[],
  span: Span,
): Anchor[] {
  const leftCounts = countUnique(left, span.leftStart, span.leftEnd);
  const rightCounts = countUnique(right, span.rightStart, span.rightEnd);
  const rightIndices = collectIndices(right, span.rightStart, span.rightEnd);
  const candidates: Anchor[] = [];
  for (let index = span.leftStart; index < span.leftEnd; index += 1) {
    const line = left[index] ?? "";
    if (leftCounts.get(line) !== 1 || rightCounts.get(line) !== 1) {
      continue;
    }
    const rightIndex = rightIndices.get(line);
    if (rightIndex !== undefined) {
      candidates.push({ leftIndex: index, rightIndex });
    }
  }
  return longestIncreasing(candidates);
}
