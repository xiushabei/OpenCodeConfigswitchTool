/** Local, screen-owned views of the chat backend contract. */

export type ChatSort = "size" | "updated" | "created";

export const CHAT_SORTS: readonly ChatSort[] = ["size", "updated", "created"];

export interface ChatDiscovery {
  paths: string[];
  searchedAllDrives: boolean;
  warnings: string[];
}

export interface ChatSession {
  id: string;
  title: string;
  directory: string;
  parentId: string | null;
  created: number;
  updated: number;
  sizeBytes: number;
  messageCount: number;
}

export interface ChatSessionList {
  schema: "v1" | "v2";
  sessions: ChatSession[];
  /** Actual length of the main database file. */
  databaseBytes: number;
  /** Actual length of the separate WAL file, 0 when it does not exist. */
  walBytes: number;
  warnings: string[];
}

export interface ChatPart {
  type: string;
  text: string;
  truncated: boolean;
}

export interface ChatMessage {
  id: string;
  role: string;
  created: number | null;
  parts: ChatPart[];
}

export interface ChatReaderData {
  id: string;
  title: string;
  messages: ChatMessage[];
  truncated: boolean;
  warnings: string[];
}

export interface ChatDeletePreview {
  sessionIds: string[];
  titles: string[];
  sizeBytes: number;
  blockedReason: string | null;
}

export interface ChatDeleteResult {
  deletedSessionIds: string[];
  backup: string;
  message: string;
}

export type ChatTone = "neutral" | "success" | "error";
