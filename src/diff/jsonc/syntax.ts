import type { JsoncSyntaxError } from "./types";

export class ParseError extends Error {
  readonly index: number;

  constructor(message: string, index: number) {
    super(message);
    this.name = "ParseError";
    this.index = index;
  }
}

export function fail(message: string, index: number): never {
  throw new ParseError(message, index);
}

export function positionAt(
  text: string,
  index: number,
): { readonly line: number; readonly column: number } {
  const offset = text.charCodeAt(0) === 0xfeff ? 1 : 0;
  const safe = Math.max(offset, Math.min(index, text.length));
  const before = text.slice(offset, safe);
  const line = before.split(/\r\n|\r|\n/).length;
  const lastBreak = Math.max(
    before.lastIndexOf("\n"),
    before.lastIndexOf("\r"),
  );
  return { line, column: before.length - lastBreak };
}

export function describeJsoncError(error: JsoncSyntaxError): string {
  return `JSON 语法错误（第 ${error.line} 行，第 ${error.column} 列）：${error.message}`;
}
