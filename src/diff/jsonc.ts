/**
 * Lossless JSONC reader for display-side comparison.
 *
 * The parser accepts comments, trailing commas and a UTF-8 BOM, keeps numbers as
 * exact decimal lexemes (never through IEEE-754 `Number`), and canonicalises object
 * keys into a stable order. Nothing here rewrites editor content or write bytes;
 * callers use the canonical text only to compare and render.
 */

export { parseJsonc } from "./jsonc/parse";
export { jsoncFieldLines } from "./jsonc/fields";
export { describeJsoncError } from "./jsonc/syntax";
export { isJsoncObject } from "./jsonc/types";
export type {
  JsoncEntry,
  JsoncNode,
  JsoncResult,
  JsoncSyntaxError,
} from "./jsonc/types";
