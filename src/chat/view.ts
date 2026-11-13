import { errorMessage } from "../tauri";
import {
  deleteChatSession,
  discoverChatDatabases,
  listChatSessions,
  previewChatDelete,
  readChatSession,
} from "./api";
import { createChatDeleteDialog } from "./delete-dialog";
import { createChatDialogShell } from "./dialog-shell";
import { createChatListPane } from "./list-pane";
import { chatNotice } from "./notice";
import { createChatReaderPane } from "./reader-pane";
import type {
  ChatDiscovery,
  ChatSession,
  ChatSort,
  ChatTone,
} from "./types";

export interface ChatView {
  element: HTMLDialogElement;
  open(): void;
}

export function createChatView(): ChatView {
  let sort: ChatSort = "updated";
  let descending = true;
  let selected: ChatSession | null = null;
  let loadedPath: string | null = null;
  let discovery: ChatDiscovery | null = null;
  let discoveryStarted = false;
  let pendingNotice: { text: string; tone: ChatTone } | null = null;
  let busy = false;
  let listToken = 0;
  let readerToken = 0;
  let opener: HTMLElement | null = null;

  const listPane = createChatListPane({
    onLoad: (_path, nextSort, nextDescending) => {
      sort = nextSort;
      descending = nextDescending;
      void loadSessions();
    },
    onSelect: (session) => {
      selectSession(session);
    },
  });
  const readerPane = createChatReaderPane();
  const shell = createChatDialogShell({
    list: listPane.element,
    reader: readerPane.element,
    onDelete: () => {
      void requestDelete();
    },
    onClose: closeDialog,
  });
  const dialog = shell.dialog;

  const deleteDialog = createChatDeleteDialog((result) => {
    pendingNotice = {
      text:
        result.backup.trim().length > 0
          ? `${result.message}\n备份：${result.backup}`
          : result.message,
      tone: "success",
    };
    clearSelection();
    void loadSessions();
  });
  document.body.append(deleteDialog.element);

  function currentPath(): string {
    return listPane.getPath().trim();
  }

  function updateDeleteButton(): void {
    shell.setDeleteEnabled(
      selected !== null && !busy,
      selected === null ? "先在左侧选择一个会话" : "预览并删除所选会话",
    );
  }

  function setBusy(next: boolean): void {
    busy = next;
    listPane.setBusy(next);
    updateDeleteButton();
  }

  function clearSelection(): void {
    selected = null;
    readerToken += 1;
    listPane.setSelected(null);
    readerPane.showPrompt();
    updateDeleteButton();
  }

  async function loadSessions(): Promise<void> {
    const path = currentPath();
    if (path.length === 0) {
      listPane.showStatus("请填写数据库文件路径。", "error");
      listPane.focusPath();
      return;
    }
    const token = ++listToken;
    const previousPath = loadedPath;
    loadedPath = null;
    setBusy(true);
    listPane.showLoading();
    try {
      const data = await listChatSessions(path, sort, descending);
      if (token !== listToken) {
        return;
      }
      loadedPath = path;
      const stillSelected =
        previousPath !== path || selected === null
          ? null
          : (data.sessions.find((session) => session.id === selected?.id) ??
            null);
      if (stillSelected === null) {
        clearSelection();
      } else {
        selected = stillSelected;
      }
      listPane.showSessions(data, selected?.id ?? null);
      if (pendingNotice !== null) {
        listPane.showNotice(pendingNotice.text, pendingNotice.tone);
        pendingNotice = null;
      } else {
        listPane.showNotice(chatNotice(discovery, data.warnings), "neutral");
      }
    } catch (error: unknown) {
      if (token === listToken) {
        loadedPath = null;
        clearSelection();
        listPane.showError(`读取会话失败：${errorMessage(error)}`);
        if (pendingNotice !== null) {
          listPane.showNotice(pendingNotice.text, pendingNotice.tone);
          pendingNotice = null;
        }
      }
    } finally {
      if (token === listToken) {
        setBusy(false);
      }
    }
  }

  function selectSession(session: ChatSession): void {
    if (loadedPath === null) {
      return;
    }
    selected = session;
    listPane.setSelected(session.id);
    updateDeleteButton();
    const path = loadedPath;
    const token = ++readerToken;
    readerPane.showLoading(session.title);
    readChatSession(path, session.id)
      .then((data) => {
        if (token === readerToken && selected?.id === session.id) {
          readerPane.showReader(data);
        }
      })
      .catch((error: unknown) => {
        if (token === readerToken && selected?.id === session.id) {
          readerPane.showError(`读取会话失败：${errorMessage(error)}`);
        }
      });
  }

  async function runDiscovery(): Promise<void> {
    listPane.setDiscoveryBusy(true);
    try {
      const result = await discoverChatDatabases();
      discovery = result;
      listPane.setDiscovery(result);
      listPane.showNotice(chatNotice(discovery, []), "neutral");
      if (result.paths.length === 0) {
        listPane.showPlaceholder("未自动找到 opencode.db，可手动填写路径后加载。");
        listPane.showStatus("未找到数据库文件。", "neutral");
        return;
      }
      if (currentPath().length > 0) {
        void loadSessions();
      }
    } catch (error: unknown) {
      discoveryStarted = false;
      listPane.showDiscoveryError(errorMessage(error));
      listPane.showNotice(null, "neutral");
    } finally {
      listPane.setDiscoveryBusy(false);
      if (dialog.open) {
        listPane.focusPath();
      }
    }
  }

  async function requestDelete(): Promise<void> {
    const session = selected;
    const path = loadedPath;
    if (session === null || busy || path === null) {
      return;
    }
    setBusy(true);
    listPane.showStatus("正在检查删除条件…", "neutral");
    try {
      const preview = await previewChatDelete(path, session.id);
      listPane.showStatus("已核对删除范围，请在确认窗口再次确认。", "neutral");
      deleteDialog.open(
        {
          title: session.title,
          sessionIds: preview.sessionIds,
          titles: preview.titles,
          sizeBytes: preview.sizeBytes,
          blockedReason: preview.blockedReason,
        },
        () => deleteChatSession(path, session.id, preview.sessionIds),
      );
    } catch (error: unknown) {
      listPane.showStatus(
        `无法检查删除条件：${errorMessage(error)}`,
        "error",
      );
    } finally {
      setBusy(false);
    }
  }

  function closeDialog(): void {
    if (dialog.open) {
      dialog.close("close");
    }
  }

  dialog.addEventListener("close", () => {
    if (opener !== null) {
      opener.focus();
    }
    opener = null;
  });

  updateDeleteButton();

  function open(): void {
    opener =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    dialog.returnValue = "";
    if (!dialog.open) {
      dialog.showModal();
    }
    if (!discoveryStarted) {
      discoveryStarted = true;
      void runDiscovery();
    } else {
      listPane.focusPath();
    }
  }

  return { element: dialog, open };
}
