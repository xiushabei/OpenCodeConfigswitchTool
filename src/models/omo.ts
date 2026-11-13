import { checkJsonObject } from "../drafts";
import { OMO_SECTIONS } from "./types";
import type { ModelAssignment, OmoSection } from "./types";

export type AssignmentRead =
  | { readonly ok: true; readonly rows: readonly ModelAssignment[] }
  | { readonly ok: false; readonly message: string };

export type AssignmentLookup =
  | {
      readonly ok: true;
      readonly kind: "missing" | "no-model" | "has-model" | "non-object";
      readonly wrapped: boolean;
    }
  | { readonly ok: false; readonly message: string };

const FORBIDDEN_KEYS: ReadonlySet<string> = new Set([
  "__proto__",
  "prototype",
  "constructor",
]);

const WRAPPER_KEY = "[opencode]";

type RootRead =
  | { readonly ok: true; readonly root: Record<string, unknown> }
  | { readonly ok: false; readonly message: string };

type HostRead =
  | {
      readonly ok: true;
      readonly host: Record<string, unknown>;
      readonly wrapped: boolean;
    }
  | { readonly ok: false; readonly message: string };

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Display path prefix for an assignment; wrapped configs live under `[opencode]`. */
export function assignmentPrefix(
  section: OmoSection,
  wrapped: boolean,
): string {
  return wrapped ? `${WRAPPER_KEY}.${section}` : section;
}

function readRoot(content: string): RootRead {
  const check = checkJsonObject(content);
  if (!check.ok) {
    return { ok: false, message: check.message };
  }
  const parsed: unknown = JSON.parse(content);
  if (!isRecord(parsed)) {
    return { ok: false, message: "顶层必须是 JSON 对象（{ … }）。" };
  }
  return { ok: true, root: parsed };
}

/**
 * Selects the object that holds the OMO sections. A present `[opencode]`
 * wrapper wins even when it is invalid, so a broken wrapper can never
 * silently fall back to leftover flat root keys.
 */
function readHost(root: Record<string, unknown>): HostRead {
  const wrapper = root[WRAPPER_KEY];
  if (wrapper === undefined) {
    return { ok: true, host: root, wrapped: false };
  }
  if (!isRecord(wrapper)) {
    return {
      ok: false,
      message: `“${WRAPPER_KEY}” 必须是对象（{ … }）。`,
    };
  }
  return { ok: true, host: wrapper, wrapped: true };
}

function sectionOf(
  host: Record<string, unknown>,
  section: OmoSection,
  wrapped: boolean,
): { readonly ok: true; readonly group: Record<string, unknown> | null } | {
  readonly ok: false;
  readonly message: string;
} {
  const group = host[section];
  if (group === undefined) {
    return { ok: true, group: null };
  }
  if (!isRecord(group)) {
    return {
      ok: false,
      message: `“${assignmentPrefix(section, wrapped)}” 必须是对象（{ … }）。`,
    };
  }
  return { ok: true, group };
}

export function readAssignments(content: string): AssignmentRead {
  const read = readRoot(content);
  if (!read.ok) {
    return read;
  }
  const host = readHost(read.root);
  if (!host.ok) {
    return host;
  }
  const rows: ModelAssignment[] = [];
  for (const section of OMO_SECTIONS) {
    const current = sectionOf(host.host, section, host.wrapped);
    if (!current.ok) {
      return current;
    }
    if (current.group === null) {
      continue;
    }
    for (const [key, value] of Object.entries(current.group)) {
      if (!isRecord(value)) {
        continue;
      }
      const model = value["model"];
      if (model !== undefined && typeof model !== "string") {
        return {
          ok: false,
          message: `${assignmentPrefix(section, host.wrapped)}.${key}.model 应为字符串。`,
        };
      }
      rows.push({
        section,
        key,
        model: model ?? null,
        wrapped: host.wrapped,
      });
    }
  }
  return { ok: true, rows };
}

export function lookupAssignment(
  content: string,
  section: OmoSection,
  key: string,
): AssignmentLookup {
  const read = readRoot(content);
  if (!read.ok) {
    return read;
  }
  const host = readHost(read.root);
  if (!host.ok) {
    return host;
  }
  const current = sectionOf(host.host, section, host.wrapped);
  if (!current.ok) {
    return current;
  }
  const wrapped = host.wrapped;
  if (current.group === null || !Object.hasOwn(current.group, key)) {
    return { ok: true, kind: "missing", wrapped };
  }
  const value = current.group[key];
  if (!isRecord(value)) {
    return { ok: true, kind: "non-object", wrapped };
  }
  const model = value["model"];
  if (model === undefined) {
    return { ok: true, kind: "no-model", wrapped };
  }
  if (typeof model !== "string") {
    return {
      ok: false,
      message: `${assignmentPrefix(section, wrapped)}.${key}.model 应为字符串。`,
    };
  }
  return { ok: true, kind: "has-model", wrapped };
}

export function validateAssignmentKey(key: string): string | null {
  if (key.trim().length === 0) {
    return "请填写 agent 或 category 的键名。";
  }
  if (FORBIDDEN_KEYS.has(key)) {
    return `键名“${key}”会触发原型链语义，请换一个名称。`;
  }
  return null;
}
