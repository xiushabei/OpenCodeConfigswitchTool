import { icon } from "../icons/icons";
import { button } from "../ui/controls";
import { create, replaceChildren } from "../ui/dom";
import { formatBytes, sortLabel } from "./format";
import { createSessionListView } from "./session-list";
import { CHAT_SORTS } from "./types";
import type {
  ChatDiscovery,
  ChatSession,
  ChatSessionList,
  ChatSort,
  ChatTone,
} from "./types";

export interface ChatListOptions {
  onLoad(path: string, sort: ChatSort, descending: boolean): void;
  onSelect(session: ChatSession): void;
}

export interface ChatListView {
  element: HTMLElement;
  getPath(): string;
  focusPath(): void;
  setDiscovery(result: ChatDiscovery): string | null;
  setDiscoveryBusy(busy: boolean): void;
  showDiscoveryError(message: string): void;
  showLoading(): void;
  showError(message: string): void;
  showPlaceholder(text: string): void;
  showSessions(data: ChatSessionList, selectedId: string | null): void;
  setSelected(id: string | null): void;
  showStatus(text: string, tone: ChatTone): void;
  showNotice(text: string | null, tone: ChatTone): void;
  setBusy(busy: boolean): void;
}

export function createChatListPane(options: ChatListOptions): ChatListView {
  let sort: ChatSort = "updated";
  let descending = true;
  let busy = false;
  let discovering = false;
  let listLoaded = false;

  const pathLabel = create("label", "field__label", ["数据库文件"]);
  pathLabel.htmlFor = "chat-db-path";
  const pathSelect = create("select", "field__control chat-source__select");
  pathSelect.hidden = true;
  pathSelect.setAttribute("aria-label", "检测到的聊天数据库");
  const pathInput = create("input", "field__control chat-source__path");
  pathInput.type = "text";
  pathInput.id = "chat-db-path";
  pathInput.autocomplete = "off";
  pathInput.spellcheck = false;
  pathInput.placeholder =
    "例如 C:\\Users\\<用户>\\.local\\share\\opencode\\opencode.db";
  const loadButton = button("加载会话", "outlined", "refresh");
  loadButton.classList.add("chat-source__load");
  const support = create("span", "field__support", [
    "只读打开。检测顺序：OPENCODE_DB、CLI 路径、默认目录；全部未命中时才扫描本机驱动器。",
  ]);
  const sizeSupport = create("span", "field__support", [
    "会话共享同一数据库，没有独立的会话文件；内容体积为消息与 part 的字节合计，文件体积为数据库主文件实际长度，WAL 单独列出。",
  ]);
  const controls = create("div", "chat-source__controls", [
    pathSelect,
    pathInput,
    loadButton,
  ]);
  const source = create("div", "chat-source", [
    pathLabel,
    controls,
    support,
    sizeSupport,
  ]);

  const sortGroup = create("div", "segmented");
  sortGroup.setAttribute("role", "group");
  sortGroup.setAttribute("aria-label", "会话排序字段");
  const sortButtons = new Map<ChatSort, HTMLButtonElement>();
  for (const value of CHAT_SORTS) {
    const item = create("button", "segmented__item", [
      create("span", "segmented__check", [icon("check", 12)]),
      create("span", "segmented__label", [sortLabel(value)]),
    ]);
    item.type = "button";
    item.setAttribute("aria-pressed", "false");
    item.addEventListener("click", () => {
      if (busy || discovering || sort === value) {
        return;
      }
      sort = value;
      syncSort();
      if (listLoaded) {
        requestLoad();
      }
    });
    sortButtons.set(value, item);
    sortGroup.append(item);
  }
  const direction = button("降序", "text");
  direction.classList.add("chat-sort__direction");
  const sortRow = create("div", "chat-sort", [sortGroup, direction]);

  const status = create("p", "chat-status", ["选择数据库后加载会话。"]);
  status.setAttribute("role", "status");
  status.setAttribute("aria-live", "polite");
  const notice = create("p", "chat-notice");
  notice.hidden = true;

  const sessionList = createSessionListView(options.onSelect);
  const element = create("div", "chat-side", [
    source,
    sortRow,
    status,
    notice,
    sessionList.element,
  ]);

  function setStatus(text: string, tone: ChatTone): void {
    status.textContent = text;
    status.setAttribute("data-tone", tone);
  }

  function syncControls(): void {
    const disabled = busy || discovering;
    pathSelect.disabled = disabled;
    pathInput.disabled = disabled;
    loadButton.disabled = disabled;
    for (const item of sortButtons.values()) {
      item.disabled = disabled;
    }
    direction.disabled = disabled;
    sessionList.setDisabled(disabled);
  }

  function syncSort(): void {
    for (const [value, item] of sortButtons) {
      item.setAttribute("aria-pressed", String(value === sort));
    }
    direction.textContent = descending ? "降序" : "升序";
    direction.title = `当前按${descending ? "降序" : "升序"}排列，点击切换`;
    direction.setAttribute("aria-label", direction.title);
  }

  function requestLoad(): void {
    if (busy || discovering) {
      return;
    }
    const path = pathInput.value.trim();
    if (path.length === 0) {
      setStatus("请填写或选择数据库文件路径。", "error");
      pathInput.focus();
      return;
    }
    options.onLoad(path, sort, descending);
  }

  loadButton.addEventListener("click", requestLoad);
  pathInput.addEventListener("keydown", (event) => {
    if (event.key === "Enter") {
      event.preventDefault();
      requestLoad();
    }
  });
  pathSelect.addEventListener("change", () => {
    if (pathSelect.value.length === 0) {
      return;
    }
    pathInput.value = pathSelect.value;
    requestLoad();
  });
  direction.addEventListener("click", () => {
    if (busy || discovering) {
      return;
    }
    descending = !descending;
    syncSort();
    if (listLoaded) {
      requestLoad();
    }
  });

  function setDiscovery(result: ChatDiscovery): string | null {
    pathSelect.hidden = result.paths.length === 0;
    const choices: HTMLOptionElement[] = [];
    if (result.paths.length > 0) {
      const detected = create("option", "", [
        `检测到 ${result.paths.length} 个数据库`,
      ]);
      detected.value = "";
      detected.disabled = true;
      choices.push(detected);
    }
    for (const path of result.paths) {
      const option = create("option", "", [path]);
      option.value = path;
      choices.push(option);
    }
    replaceChildren(pathSelect, choices);

    const first = result.paths[0] ?? null;
    if (first === null) {
      return null;
    }
    pathSelect.value = first;
    if (pathInput.value.trim().length > 0) {
      return null;
    }
    pathInput.value = first;
    return first;
  }

  function showSessions(
    data: ChatSessionList,
    selectedId: string | null,
  ): void {
    listLoaded = true;
    const volumes: string[] = [];
    if (data.databaseBytes > 0) {
      volumes.push(`文件体积 ${formatBytes(data.databaseBytes)}`);
    }
    if (data.walBytes > 0) {
      volumes.push(`WAL ${formatBytes(data.walBytes)}`);
    }
    const volume = volumes.length > 0 ? ` · ${volumes.join(" · ")}` : "";
    setStatus(
      `已读取 ${data.sessions.length} 个会话 · 结构 ${data.schema}${volume}`,
      "neutral",
    );
    sessionList.showSessions(data, selectedId, busy || discovering);
  }

  syncSort();

  return {
    element,
    getPath: () => pathInput.value,
    focusPath: () => {
      pathInput.focus();
      pathInput.select();
    },
    setDiscovery,
    setDiscoveryBusy: (next) => {
      discovering = next;
      syncControls();
      if (next) {
        setStatus("正在检测数据库…", "neutral");
      }
    },
    showDiscoveryError: (message) => {
      setStatus(`检测数据库失败：${message}`, "error");
      sessionList.showMessage("可手动填写数据库路径后加载。");
    },
    showLoading: () => {
      listLoaded = true;
      setStatus("正在读取会话…", "neutral");
      sessionList.showMessage("正在读取…");
    },
    showError: (message) => {
      listLoaded = true;
      setStatus(message, "error");
      sessionList.showMessage("读取失败。请检查路径或数据库结构后重试。");
    },
    showPlaceholder: (text) => {
      sessionList.showMessage(text);
    },
    showSessions,
    setSelected: sessionList.setSelected,
    showStatus: setStatus,
    showNotice: (text, tone) => {
      notice.textContent = text ?? "";
      notice.hidden = text === null || text.trim().length === 0;
      notice.setAttribute("data-tone", tone);
    },
    setBusy: (next) => {
      busy = next;
      syncControls();
    },
  };
}
