export type { DiffKind, DiffLine, DiffResult } from "./diff/model";
export { MAX_RENDER_LINES, MAX_SOURCE_LINES, splitLines } from "./diff/model";
export { diffLines } from "./diff/lines";
export {
  canonicalConfig,
  configsEquivalent,
  describeConfigError,
  diffConfigs,
} from "./diff/config";
export type { CanonicalConfig, ConfigDiff } from "./diff/config";
export {
  describeJsoncError,
  isJsoncObject,
  jsoncFieldLines,
  parseJsonc,
} from "./diff/jsonc";
export type {
  JsoncEntry,
  JsoncNode,
  JsoncResult,
  JsoncSyntaxError,
} from "./diff/jsonc";
