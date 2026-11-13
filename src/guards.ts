import { isTarget, isTheme } from "./types";
import type {
  ApplyResult,
  Preset,
  Settings,
  Snapshot,
  TargetState,
} from "./types";

/** Raised when a backend payload does not match the shared contract. */
export class ContractError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ContractError";
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isUnknownArray(value: unknown): value is readonly unknown[] {
  return Array.isArray(value);
}

function requiredRecord(value: unknown, path: string): Record<string, unknown> {
  if (!isRecord(value)) {
    throw new ContractError(`${path} 应为对象`);
  }
  return value;
}

function requiredArray(value: unknown, path: string): readonly unknown[] {
  if (!isUnknownArray(value)) {
    throw new ContractError(`${path} 应为数组`);
  }
  return value;
}

function requiredString(value: unknown, path: string): string {
  if (typeof value !== "string") {
    throw new ContractError(`${path} 应为字符串`);
  }
  return value;
}

function nullableString(value: unknown, path: string): string | null {
  if (value === null) {
    return null;
  }
  if (typeof value !== "string") {
    throw new ContractError(`${path} 应为字符串或 null`);
  }
  return value;
}

function requiredBoolean(value: unknown, path: string): boolean {
  if (typeof value !== "boolean") {
    throw new ContractError(`${path} 应为布尔值`);
  }
  return value;
}

export function parsePreset(value: unknown, path: string): Preset {
  const record = requiredRecord(value, path);
  const target = record["target"];
  if (!isTarget(target)) {
    throw new ContractError(`${path}.target 不是已知目标`);
  }
  return {
    id: requiredString(record["id"], `${path}.id`),
    name: requiredString(record["name"], `${path}.name`),
    description: requiredString(record["description"], `${path}.description`),
    target,
    content: requiredString(record["content"], `${path}.content`),
  };
}

export function parseSettings(value: unknown): Settings {
  const record = requiredRecord(value, "settings");
  const targets = requiredRecord(record["targets"], "settings.targets");
  const theme = record["theme"];
  if (!isTheme(theme)) {
    throw new ContractError("settings.theme 不是已知主题");
  }
  return {
    targets: {
      opencode: requiredString(targets["opencode"], "settings.targets.opencode"),
      omo: requiredString(targets["omo"], "settings.targets.omo"),
    },
    theme,
  };
}

function parseTargetState(value: unknown, path: string): TargetState {
  const record = requiredRecord(value, path);
  const target = record["target"];
  if (!isTarget(target)) {
    throw new ContractError(`${path}.target 不是已知目标`);
  }
  const activePresetIds = requiredArray(
    record["activePresetIds"],
    `${path}.activePresetIds`,
  ).map((id, index) => requiredString(id, `${path}.activePresetIds[${index}]`));
  return {
    target,
    path: requiredString(record["path"], `${path}.path`),
    content: nullableString(record["content"], `${path}.content`),
    error: nullableString(record["error"], `${path}.error`),
    activePresetIds,
  };
}

export function parseSnapshot(value: unknown): Snapshot {
  const record = requiredRecord(value, "snapshot");
  const presets = requiredArray(record["presets"], "snapshot.presets").map(
    (preset, index) => parsePreset(preset, `snapshot.presets[${index}]`),
  );
  const targets = requiredArray(record["targets"], "snapshot.targets").map(
    (target, index) => parseTargetState(target, `snapshot.targets[${index}]`),
  );
  const startupWarnings = requiredArray(
    record["startupWarnings"],
    "snapshot.startupWarnings",
  ).map((warning, index) =>
    requiredString(warning, `snapshot.startupWarnings[${index}]`),
  );
  return {
    settings: parseSettings(record["settings"]),
    presets,
    targets,
    root: requiredString(record["root"], "snapshot.root"),
    startupWarnings,
    canRestoreStartup: requiredBoolean(
      record["canRestoreStartup"],
      "snapshot.canRestoreStartup",
    ),
  };
}

export function parseApplyResult(value: unknown): ApplyResult {
  const record = requiredRecord(value, "applyResult");
  return {
    snapshot: parseSnapshot(record["snapshot"]),
    backup: nullableString(record["backup"], "applyResult.backup"),
    message: requiredString(record["message"], "applyResult.message"),
  };
}

export function parseExportPath(value: unknown): string {
  const path = requiredString(value, "exportResult");
  if (path.trim().length === 0) {
    throw new ContractError("exportResult 为空路径");
  }
  return path;
}

export function parseImportContent(value: unknown): string {
  const content = requiredString(value, "importContent");
  if (content.trim().length === 0) {
    throw new ContractError("importContent 为空内容");
  }
  return content;
}
