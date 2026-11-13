import type { Draft, Preset, Target } from "./types";

export type JsonCheck = { ok: true } | { ok: false; message: string };

export type DraftValidation =
  | { ok: true; preset: Preset }
  | { ok: false; field: "name" | "content"; message: string };

export function draftFromPreset(preset: Preset): Draft {
  return {
    id: preset.id,
    name: preset.name,
    description: preset.description,
    target: preset.target,
    content: preset.content,
  };
}

export function blankDraft(target: Target): Draft {
  return { id: "", name: "", description: "", target, content: "" };
}

export function draftsEqual(left: Draft | null, right: Draft | null): boolean {
  if (left === null || right === null) {
    return left === right;
  }
  return (
    left.id === right.id &&
    left.name === right.name &&
    left.description === right.description &&
    left.target === right.target &&
    left.content === right.content
  );
}

function lineColumnAt(text: string, position: number): string {
  const safe = Math.max(0, Math.min(position, text.length));
  const before = text.slice(0, safe);
  const line = before.split("\n").length;
  const lastBreak = before.lastIndexOf("\n");
  const column = safe - lastBreak;
  return `第 ${line} 行，第 ${column} 列`;
}

function describeJsonError(error: unknown, text: string): string {
  const raw = error instanceof Error ? error.message : String(error);
  const match = /position\s+(\d+)/i.exec(raw);
  if (match === null) {
    return `JSON 无效：${raw}`;
  }
  const position = Number.parseInt(match[1] ?? "", 10);
  if (!Number.isFinite(position)) {
    return `JSON 无效：${raw}`;
  }
  return `JSON 无效（${lineColumnAt(text, position)}）：${raw}`;
}

export function checkJsonObject(text: string): JsonCheck {
  if (text.trim().length === 0) {
    return { ok: false, message: "内容为空，请输入 JSON 对象。" };
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch (error) {
    return { ok: false, message: describeJsonError(error, text) };
  }
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
    return { ok: false, message: "顶层必须是 JSON 对象（{ … }）。" };
  }
  return { ok: true };
}

export function validateDraft(draft: Draft): DraftValidation {
  const name = draft.name.trim();
  if (name.length === 0) {
    return { ok: false, field: "name", message: "请填写预设名称。" };
  }
  const json = checkJsonObject(draft.content);
  if (!json.ok) {
    return { ok: false, field: "content", message: json.message };
  }
  return {
    ok: true,
    preset: {
      id: draft.id,
      name,
      description: draft.description.trim(),
      target: draft.target,
      content: draft.content,
    },
  };
}
