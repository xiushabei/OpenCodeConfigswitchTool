import { create, replaceChildren } from "../ui/dom";
import { createSessionRow } from "./session-row";
import type { ChatSession, ChatSessionList } from "./types";

export interface SessionListView {
  element: HTMLElement;
  showSessions(
    data: ChatSessionList,
    selectedId: string | null,
    disabled: boolean,
  ): void;
  setSelected(id: string | null): void;
  showMessage(text: string): void;
  setDisabled(disabled: boolean): void;
}

export function createSessionListView(
  onSelect: (session: ChatSession) => void,
): SessionListView {
  const element = create("div", "chat-list");
  element.setAttribute("role", "group");
  element.setAttribute("aria-label", "聊天会话");
  const rowButtons = new Map<string, HTMLButtonElement>();

  function setSelected(id: string | null): void {
    for (const [sessionId, row] of rowButtons) {
      row.setAttribute("aria-pressed", String(sessionId === id));
    }
  }

  function showMessage(text: string): void {
    rowButtons.clear();
    replaceChildren(element, [create("p", "chat-list__empty", [text])]);
  }

  function setDisabled(disabled: boolean): void {
    for (const row of rowButtons.values()) {
      row.disabled = disabled;
    }
  }

  function showSessions(
    data: ChatSessionList,
    selectedId: string | null,
    disabled: boolean,
  ): void {
    if (data.sessions.length === 0) {
      showMessage("该数据库没有会话。");
      return;
    }

    const byId = new Map<string, ChatSession>();
    const parents = new Set<string>();
    for (const session of data.sessions) {
      byId.set(session.id, session);
      if (session.parentId !== null) {
        parents.add(session.parentId);
      }
    }

    const rows: HTMLButtonElement[] = [];
    rowButtons.clear();
    for (const session of data.sessions) {
      const parent =
        session.parentId === null ? null : byId.get(session.parentId);
      const row = createSessionRow(session, {
        hasChildren: parents.has(session.id),
        parentTitle: parent?.title ?? null,
      });
      row.disabled = disabled;
      row.addEventListener("click", () => {
        if (!row.disabled) {
          onSelect(session);
        }
      });
      rowButtons.set(session.id, row);
      rows.push(row);
    }
    replaceChildren(element, rows);
    setSelected(selectedId);
  }

  return { element, showSessions, setSelected, showMessage, setDisabled };
}
