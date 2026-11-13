export type DiffKind = "same" | "add" | "remove";

export interface DiffLine {
  kind: DiffKind;
  text: string;
  oldLine: number | null;
  newLine: number | null;
}

export interface DiffResult {
  lines: DiffLine[];
  added: number;
  removed: number;
  identical: boolean;
  truncated: boolean;
}

export interface Counters {
  added: number;
  removed: number;
}

export interface Span {
  leftStart: number;
  leftEnd: number;
  rightStart: number;
  rightEnd: number;
}

export interface Anchor {
  leftIndex: number;
  rightIndex: number;
}

export const MAX_SOURCE_LINES = 20000;
export const MAX_RENDER_LINES = 1500;
export const MAX_DP_CELLS = 40000;

/** Splits on LF after normalising CRLF/CR; a trailing newline keeps its empty line. */
export function splitLines(text: string): string[] {
  if (text.length === 0) {
    return [];
  }
  return text.replace(/\r\n/g, "\n").replace(/\r/g, "\n").split("\n");
}
