import type {
  ChatDeletePreview,
  ChatDeleteResult,
  ChatDiscovery,
  ChatMessage,
  ChatPart,
  ChatReaderData,
  ChatSession,
  ChatSessionList,
} from "./types";

/** Raised when a chat command payload does not match the agreed contract. */
export class ChatContractError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ChatContractError";
  }
}

function readRecord(value: unknown, path: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new ChatContractError(`${path} 应为对象`);
  }
  return value as Record<string, unknown>;
}

function readArray(value: unknown, path: string): readonly unknown[] {
  if (!Array.isArray(value)) {
    throw new ChatContractError(`${path} 应为数组`);
  }
  return value;
}

function readString(
  value: unknown,
  path: string,
  allowEmpty: boolean = false,
): string {
  if (typeof value !== "string") {
    throw new ChatContractError(`${path} 应为字符串`);
  }
  if (!allowEmpty && value.trim().length === 0) {
    throw new ChatContractError(`${path} 不应为空`);
  }
  return value;
}

function readNullableString(value: unknown, path: string): string | null {
  if (value === null) {
    return null;
  }
  return readString(value, path);
}

function readStringArray(
  value: unknown,
  path: string,
  allowEmpty: boolean = false,
): string[] {
  return readArray(value, path).map((entry, index) =>
    readString(entry, `${path}[${index}]`, allowEmpty),
  );
}

function readBoolean(value: unknown, path: string): boolean {
  if (typeof value !== "boolean") {
    throw new ChatContractError(`${path} 应为布尔值`);
  }
  return value;
}

function readNumber(
  value: unknown,
  path: string,
  integer: boolean = false,
): number {
  if (
    typeof value !== "number" ||
    !Number.isFinite(value) ||
    value < 0 ||
    (integer && !Number.isSafeInteger(value))
  ) {
    throw new ChatContractError(`${path} 应为非负数值`);
  }
  return value;
}

function parseSession(value: unknown, path: string): ChatSession {
  const record = readRecord(value, path);
  return {
    id: readString(record["id"], `${path}.id`),
    title: readString(record["title"], `${path}.title`, true),
    directory: readString(record["directory"], `${path}.directory`, true),
    parentId: readNullableString(record["parentId"], `${path}.parentId`),
    created: readNumber(record["created"], `${path}.created`),
    updated: readNumber(record["updated"], `${path}.updated`),
    sizeBytes: readNumber(record["sizeBytes"], `${path}.sizeBytes`, true),
    messageCount: readNumber(
      record["messageCount"],
      `${path}.messageCount`,
      true,
    ),
  };
}

function parsePart(value: unknown, path: string): ChatPart {
  const record = readRecord(value, path);
  return {
    type: readString(record["type"], `${path}.type`),
    text: readString(record["text"], `${path}.text`, true),
    truncated: readBoolean(record["truncated"], `${path}.truncated`),
  };
}

function parseMessage(value: unknown, path: string): ChatMessage {
  const record = readRecord(value, path);
  const created = record["created"];
  return {
    id: readString(record["id"], `${path}.id`),
    role: readString(record["role"], `${path}.role`),
    created:
      created === null ? null : readNumber(created, `${path}.created`),
    parts: readArray(record["parts"], `${path}.parts`).map((part, index) =>
      parsePart(part, `${path}.parts[${index}]`),
    ),
  };
}

export function parseDiscovery(value: unknown): ChatDiscovery {
  const record = readRecord(value, "discovery");
  return {
    paths: readStringArray(record["paths"], "discovery.paths"),
    searchedAllDrives: readBoolean(
      record["searchedAllDrives"],
      "discovery.searchedAllDrives",
    ),
    warnings: readStringArray(record["warnings"], "discovery.warnings", true),
  };
}

export function parseSessionList(value: unknown): ChatSessionList {
  const record = readRecord(value, "sessionList");
  const schema = record["schema"];
  if (schema !== "v1" && schema !== "v2") {
    throw new ChatContractError("sessionList.schema 应为 v1 或 v2");
  }
  return {
    schema,
    sessions: readArray(record["sessions"], "sessionList.sessions").map(
      (session, index) =>
        parseSession(session, `sessionList.sessions[${index}]`),
    ),
    databaseBytes: readNumber(
      record["databaseBytes"],
      "sessionList.databaseBytes",
      true,
    ),
    walBytes: readNumber(record["walBytes"], "sessionList.walBytes", true),
    warnings: readStringArray(
      record["warnings"],
      "sessionList.warnings",
      true,
    ),
  };
}

export function parseReader(value: unknown): ChatReaderData {
  const record = readRecord(value, "reader");
  return {
    id: readString(record["id"], "reader.id"),
    title: readString(record["title"], "reader.title", true),
    messages: readArray(record["messages"], "reader.messages").map(
      (message, index) => parseMessage(message, `reader.messages[${index}]`),
    ),
    truncated: readBoolean(record["truncated"], "reader.truncated"),
    warnings: readStringArray(record["warnings"], "reader.warnings", true),
  };
}

export function parseDeletePreview(value: unknown): ChatDeletePreview {
  const record = readRecord(value, "deletePreview");
  return {
    sessionIds: readStringArray(
      record["sessionIds"],
      "deletePreview.sessionIds",
    ),
    titles: readStringArray(record["titles"], "deletePreview.titles", true),
    sizeBytes: readNumber(
      record["sizeBytes"],
      "deletePreview.sizeBytes",
      true,
    ),
    blockedReason: readNullableString(
      record["blockedReason"],
      "deletePreview.blockedReason",
    ),
  };
}

export function parseDeleteResult(value: unknown): ChatDeleteResult {
  const record = readRecord(value, "deleteResult");
  return {
    deletedSessionIds: readStringArray(
      record["deletedSessionIds"],
      "deleteResult.deletedSessionIds",
    ),
    backup: readString(record["backup"], "deleteResult.backup", true),
    message: readString(record["message"], "deleteResult.message", true),
  };
}
