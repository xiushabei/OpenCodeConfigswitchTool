import { create, replaceChildren } from "../ui/dom";
import {
  buildSessionEntries,
  createSessionRowView,
  type SessionEntry,
} from "./session-rows";
import { focusTargetForKey } from "./session-window";
import { createSessionWindowView } from "./session-window-view";
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
  const inner = create("div", "chat-list__inner");

  let entries: SessionEntry[] = [];
  let indexById = new Map<string, number>();
  let selectedId: string | null = null;
  let disabled = false;
  let activeIndex = 0;

  const rows = createSessionRowView({
    host: inner,
    entryAt: (index) => entries[index],
    state: () => ({ selectedId, disabled, activeIndex }),
    offsetOf: (index) => view.offsets()[index] ?? 0,
    onSelect: (session, index) => {
      activeIndex = index;
      rows.syncTabIndex();
      onSelect(session);
    },
  });

  const view = createSessionWindowView({
    element,
    inner,
    rows,
    entries: () => entries,
    activeIndex: () => activeIndex,
  });
  view.attach();

  function focusIndex(index: number): void {
    if (entries.length === 0) {
      return;
    }
    const next = Math.min(Math.max(index, 0), entries.length - 1);
    activeIndex = next;
    view.reveal(next);
  }

  element.addEventListener("focusin", (event) => {
    const target = event.target;
    if (!(target instanceof HTMLButtonElement)) {
      return;
    }
    if (!target.classList.contains("chat-item")) {
      return;
    }
    const id = target.dataset["sessionId"];
    if (id === undefined) {
      return;
    }
    const index = indexById.get(id);
    if (index === undefined || index === activeIndex) {
      return;
    }
    activeIndex = index;
    rows.syncTabIndex();
  });

  element.addEventListener("keydown", (event) => {
    const target = event.target;
    if (!(target instanceof HTMLElement)) {
      return;
    }
    if (!target.classList.contains("chat-item")) {
      return;
    }
    if (entries.length === 0) {
      return;
    }
    const next = focusTargetForKey({
      key: event.key,
      activeIndex,
      count: entries.length,
      offsets: view.offsets(),
      scrollTop: view.scrollTop(),
      viewport: view.viewportHeight(),
    });
    if (next === null) {
      return;
    }
    event.preventDefault();
    focusIndex(next);
  });

  function setSelected(id: string | null): void {
    if (selectedId === id) {
      return;
    }
    const previous = selectedId;
    selectedId = id;
    for (const index of rows.indices()) {
      const sessionId = entries[index]?.session.id;
      if (sessionId === previous || sessionId === id) {
        rows.applyState(index);
      }
    }
  }

  function showMessage(text: string): void {
    entries = [];
    indexById = new Map<string, number>();
    rows.clear();
    activeIndex = 0;
    view.clear();
    element.classList.remove("chat-list--windowed");
    replaceChildren(inner, []);
    replaceChildren(element, [create("p", "chat-list__empty", [text])]);
  }

  function setDisabled(next: boolean): void {
    disabled = next;
    for (const index of rows.indices()) {
      rows.applyState(index);
    }
  }

  function showSessions(
    data: ChatSessionList,
    selected: string | null,
    nextDisabled: boolean,
  ): void {
    if (data.sessions.length === 0) {
      showMessage("该数据库没有会话。");
      return;
    }

    selectedId = selected;
    disabled = nextDisabled;
    entries = buildSessionEntries(data.sessions);
    indexById = new Map(
      entries.map((entry, index) => [entry.session.id, index]),
    );
    view.reset(entries);
    activeIndex = Math.min(activeIndex, entries.length - 1);
    if (activeIndex < 0) {
      activeIndex = 0;
    }

    element.classList.add("chat-list--windowed");
    rows.clear();
    replaceChildren(inner, []);
    replaceChildren(element, [inner]);
    view.syncWidth();
    view.update();
    view.measure();
  }

  return {
    element,
    showSessions,
    setSelected,
    showMessage,
    setDisabled,
  };
}
