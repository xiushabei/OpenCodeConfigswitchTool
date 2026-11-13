import { badge } from "../ui/controls";
import { create } from "../ui/dom";
import { formatBytes, formatTime, sessionTitle } from "./format";
import type { ChatSession } from "./types";

export interface SessionRowContext {
  hasChildren: boolean;
  parentTitle: string | null;
}

export function createSessionRow(
  session: ChatSession,
  context: SessionRowContext,
): HTMLButtonElement {
  const titleRow = create("span", "chat-item__title-row");
  if (session.parentId !== null) {
    titleRow.append(badge("子会话", "target"));
  } else if (context.hasChildren) {
    titleRow.append(badge("父会话", "target"));
  }
  titleRow.append(
    create("span", "chat-item__title", [sessionTitle(session.title)]),
  );

  const meta = create("span", "chat-item__meta", [
    create("span", "", [`内容体积 ${formatBytes(session.sizeBytes)}`]),
    create("span", "", [`${session.messageCount} 条消息`]),
  ]);
  const times = create("span", "chat-item__meta chat-item__meta--sub", [
    create("span", "", [`创建 ${formatTime(session.created)}`]),
    create("span", "", [`更新 ${formatTime(session.updated)}`]),
  ]);
  if (session.parentId !== null) {
    times.append(
      create("span", "chat-item__parent", [
        `父会话：${context.parentTitle ?? session.parentId}`,
      ]),
    );
  }

  const body = create("span", "chat-item__body", [titleRow, meta, times]);
  const row = create(
    "button",
    session.parentId === null ? "chat-item" : "chat-item chat-item--child",
    [body],
  );
  row.type = "button";
  row.setAttribute("aria-pressed", "false");
  row.setAttribute("data-session-id", session.id);
  const heading = sessionTitle(session.title);
  row.title =
    session.directory.length > 0
      ? `${heading}\n${session.directory}`
      : heading;
  return row;
}
