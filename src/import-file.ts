import { parsePreset } from "./guards";
import type { Target } from "./types";

export interface ImportPayload {
  name: string;
  description: string;
  target: Target;
  content: string;
}

export type ImportParse =
  | { ok: true; payload: ImportPayload }
  | { ok: false; message: string };

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function fileBaseName(fileName: string): string {
  return fileName.replace(/\\/g, "/").split("/").pop() ?? fileName;
}

function fileStem(fileName: string): string {
  return fileBaseName(fileName).replace(/\.(json|jsonc)$/i, "").trim();
}

export function isImportableFileName(fileName: string): boolean {
  return /\.(json|jsonc)$/i.test(fileBaseName(fileName));
}

/**
 * Infers the target from well-known configuration file names. OMO names are
 * checked first because "oh-my-opencode" also contains "opencode".
 */
export function targetFromFileName(
  fileName: string,
  fallbackTarget: Target,
): Target {
  const base = fileBaseName(fileName).toLowerCase();
  if (
    base.includes("oh-my-openagent") ||
    base.includes("oh-my-opencode") ||
    /(^|[^a-z])omo([^a-z]|$)/.test(base)
  ) {
    return "omo";
  }
  if (base.includes("opencode")) {
    return "opencode";
  }
  return fallbackTarget;
}

/**
 * Accepts the backend export envelope or a raw JSON object. Raw objects keep
 * the backend-normalized text verbatim as content, so numeric literals are not
 * round-tripped through JSON.parse. Preset content from an envelope is kept
 * verbatim too.
 */
export function parseImportFile(
  text: string,
  fileName: string,
  fallbackTarget: Target,
): ImportParse {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch (error) {
    return {
      ok: false,
      message: `导入文件不是有效 JSON：${
        error instanceof Error ? error.message : "解析失败"
      }`,
    };
  }

  if (!isRecord(parsed)) {
    return { ok: false, message: "导入文件顶层必须是 JSON 对象。" };
  }

  const hasEnvelope = "formatVersion" in parsed && isRecord(parsed["preset"]);
  if (hasEnvelope) {
    if (parsed["formatVersion"] !== 1) {
      return { ok: false, message: "不支持的导出格式版本，需要 formatVersion 1。" };
    }
    try {
      const preset = parsePreset(parsed["preset"], "preset");
      const stem = fileStem(fileName);
      return {
        ok: true,
        payload: {
          name: preset.name.trim().length > 0 ? preset.name : stem || "导入的预设",
          description: preset.description,
          target: preset.target,
          content: preset.content,
        },
      };
    } catch (error) {
      return {
        ok: false,
        message:
          error instanceof Error ? error.message : "导出文件中的预设无效。",
      };
    }
  }

  return {
    ok: true,
    payload: {
      name: fileStem(fileName) || "导入的预设",
      description: "",
      target: fallbackTarget,
      content: text,
    },
  };
}
