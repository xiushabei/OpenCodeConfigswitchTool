import {
  describeJsoncError,
  isJsoncObject,
  jsoncFieldLines,
  parseJsonc,
} from "./jsonc";
import type { JsoncSyntaxError } from "./jsonc";
import { diffLines } from "./lines";
import type { DiffLine, DiffResult } from "./model";

/** Result of comparing one preset draft against the current target file. */
export interface ConfigDiff {
  /** `semantic` means both sides parsed as JSON objects and were canonicalised. */
  mode: "semantic" | "raw";
  /** Raw-text diff when invalid JSON forces the fallback; semantic diff otherwise. */
  result: DiffResult;
  targetError: JsoncSyntaxError | null;
  draftError: JsoncSyntaxError | null;
  rawIdentical: boolean;
  /** True only when both sides are valid JSON objects with equal canonical text. */
  semanticEqual: boolean;
}

export type CanonicalConfig =
  | { ok: true; fields: string }
  | { ok: false; error: JsoncSyntaxError };

function topLevelError(): JsoncSyntaxError {
  return {
    message: "顶层必须是 JSON 对象。",
    index: 0,
    line: 1,
    column: 1,
  };
}

function canonicalObject(text: string): CanonicalConfig {
  const parsed = parseJsonc(text);
  if (!parsed.ok) {
    return { ok: false, error: parsed.error };
  }
  if (!isJsoncObject(parsed.value)) {
    return { ok: false, error: topLevelError() };
  }
  return {
    ok: true,
    fields: jsoncFieldLines(parsed.value).join("\n"),
  };
}

/**
 * Canonical field lines for equality checks.
 *
 * Comments and formatting are discarded, object keys are sorted, arrays keep their
 * order, and number lexemes are normalised without converting through `Number`, so
 * integers beyond 2^53 stay exact.
 */
export function canonicalConfig(text: string): CanonicalConfig {
  return canonicalObject(text);
}

/**
 * Coordinate equality used by the diff view: both sides must be valid JSON objects
 * and their canonical text must match. Invalid input is never reported equal.
 */
export function configsEquivalent(left: string, right: string): boolean {
  const leftConfig = canonicalObject(left);
  const rightConfig = canonicalObject(right);
  return (
    leftConfig.ok &&
    rightConfig.ok &&
    leftConfig.fields === rightConfig.fields
  );
}

function withoutLineNumbers(lines: readonly DiffLine[]): DiffLine[] {
  return lines.map((line) => ({
    ...line,
    oldLine: null,
    newLine: null,
  }));
}

function rawDiff(
  targetText: string,
  draftText: string,
  targetError: JsoncSyntaxError | null,
  draftError: JsoncSyntaxError | null,
  rawIdentical: boolean,
): ConfigDiff {
  return {
    mode: "raw",
    result: diffLines(targetText, draftText),
    targetError,
    draftError,
    rawIdentical,
    semanticEqual: false,
  };
}

/**
 * Compares target JSONC with draft JSON, preferring a semantic diff.
 *
 * Valid JSON/JSONC on both sides is canonicalised before diffing, so comments,
 * whitespace and key order never produce rows. If either side is invalid, the
 * caller gets the raw line diff plus the syntax error: invalid content is never
 * silently reported as identical.
 */
export function diffConfigs(targetText: string, draftText: string): ConfigDiff {
  const rawIdentical = targetText === draftText;
  const target = canonicalObject(targetText);
  const draft = canonicalObject(draftText);

  if (!target.ok || !draft.ok) {
    return rawDiff(
      targetText,
      draftText,
      target.ok ? null : target.error,
      draft.ok ? null : draft.error,
      rawIdentical,
    );
  }

  const semanticEqual = target.fields === draft.fields;
  const diff = diffLines(target.fields, draft.fields);
  return {
    mode: "semantic",
    result: {
      ...diff,
      lines: withoutLineNumbers(diff.lines),
    },
    targetError: null,
    draftError: null,
    rawIdentical,
    semanticEqual,
  };
}

export function describeConfigError(
  side: "target" | "draft",
  error: JsoncSyntaxError,
): string {
  const label = side === "target" ? "目标文件" : "预设内容";
  return `${label}：${describeJsoncError(error)}`;
}
