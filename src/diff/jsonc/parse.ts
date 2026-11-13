import { Parser } from "./parser";
import { ParseError, positionAt } from "./syntax";
import type { JsoncResult } from "./types";

export function parseJsonc(text: string): JsoncResult {
  try {
    const value = new Parser(text).parse();
    return { ok: true, value };
  } catch (error) {
    if (error instanceof ParseError) {
      const { line, column } = positionAt(text, error.index);
      return {
        ok: false,
        error: {
          message: error.message,
          index: error.index,
          line,
          column,
        },
      };
    }
    throw error;
  }
}
