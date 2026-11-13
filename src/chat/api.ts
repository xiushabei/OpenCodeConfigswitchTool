import { invokeCommand } from "../tauri";
import {
  parseDeletePreview,
  parseDeleteResult,
  parseDiscovery,
  parseReader,
  parseSessionList,
} from "./parse";
import type {
  ChatDeletePreview,
  ChatDeleteResult,
  ChatDiscovery,
  ChatReaderData,
  ChatSessionList,
  ChatSort,
} from "./types";

const COMMANDS = {
  discover: "discover_chat_databases",
  list: "list_chat_sessions",
  read: "read_chat_session",
  previewDelete: "preview_chat_delete",
  delete: "delete_chat_session",
} as const;

export async function discoverChatDatabases(): Promise<ChatDiscovery> {
  return parseDiscovery(await invokeCommand(COMMANDS.discover));
}

export async function listChatSessions(
  path: string,
  sort: ChatSort,
  descending: boolean,
): Promise<ChatSessionList> {
  return parseSessionList(
    await invokeCommand(COMMANDS.list, { path, sort, descending }),
  );
}

export async function readChatSession(
  path: string,
  sessionId: string,
): Promise<ChatReaderData> {
  return parseReader(
    await invokeCommand(COMMANDS.read, { path, sessionId }),
  );
}

export async function previewChatDelete(
  path: string,
  sessionId: string,
): Promise<ChatDeletePreview> {
  return parseDeletePreview(
    await invokeCommand(COMMANDS.previewDelete, { path, sessionId }),
  );
}

export async function deleteChatSession(
  path: string,
  sessionId: string,
  confirmedSessionIds: readonly string[],
): Promise<ChatDeleteResult> {
  return parseDeleteResult(
    await invokeCommand(COMMANDS.delete, {
      path,
      sessionId,
      confirmedSessionIds,
    }),
  );
}
