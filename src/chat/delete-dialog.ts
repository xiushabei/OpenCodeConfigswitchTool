import { errorMessage } from "../tauri";
import { icon } from "../icons/icons";
import { button, iconButton } from "../ui/controls";
import { create, replaceChildren } from "../ui/dom";
import { formatBytes } from "./format";
import type { ChatDeleteResult } from "./types";

export interface ChatDeleteRequest {
  title: string;
  sessionIds: readonly string[];
  titles: readonly string[];
  sizeBytes: number;
  blockedReason: string | null;
}

export interface ChatDeleteDialogView {
  element: HTMLDialogElement;
  open(
    request: ChatDeleteRequest,
    execute: () => Promise<ChatDeleteResult>,
  ): void;
}

export function createChatDeleteDialog(
  onDeleted: (result: ChatDeleteResult) => void,
): ChatDeleteDialogView {
  const dialog = create("dialog", "dialog dialog--chat-delete");
  dialog.id = "chat-delete-dialog";

  const title = create("h2", "dialog__title", ["删除会话"]);
  title.id = "chat-delete-title";
  const close = iconButton("close", "取消删除");
  const head = create("div", "dialog__head", [title, close]);

  const summary = create("p", "dialog__text", [""]);
  const stats = create("p", "mono muted", [""]);
  const listTitle = create("h3", "field__label", [
    "将删除的会话（含全部子会话）",
  ]);
  const list = create("div", "chat-delete__list");
  list.setAttribute("role", "list");

  const blockedText = create("span", "", [""]);
  const blocked = create("div", "dialog__error", [
    icon("error", 18),
    blockedText,
  ]);
  blocked.hidden = true;

  const backupNote = create("p", "dialog__note", [
    "确认后会先通过 SQLite 在线备份写入项目 backups/chat 目录；备份成功后才删除，操作不可撤销。",
  ]);
  const errorText = create("span", "", [""]);
  const errorLine = create("p", "dialog__error", [errorText]);
  errorLine.hidden = true;
  errorLine.setAttribute("role", "alert");

  const body = create("div", "dialog__body", [
    summary,
    stats,
    listTitle,
    list,
    blocked,
    backupNote,
    errorLine,
  ]);

  const cancel = button("取消", "text");
  const confirm = button("确认删除并备份", "filled", "delete");
  confirm.classList.add("btn--critical");
  const actions = create("div", "dialog__actions", [cancel, confirm]);
  const form = create("div", "dialog__form", [head, body, actions]);
  dialog.append(form);
  dialog.setAttribute("aria-labelledby", "chat-delete-title");

  let busy = false;
  let locked = false;
  let execute: (() => Promise<ChatDeleteResult>) | null = null;

  function showError(message: string): void {
    errorText.textContent = message;
    errorLine.hidden = message.trim().length === 0;
  }

  function setBusy(next: boolean): void {
    busy = next;
    confirm.disabled = next || locked;
    cancel.disabled = next;
    close.disabled = next;
    dialog.setAttribute("aria-busy", String(next));
  }

  function renderPreview(request: ChatDeleteRequest): void {
    summary.textContent = `将删除会话「${request.title}」及其全部子会话。`;
    const count = request.sessionIds.length;
    const extra = count > 1 ? `（1 个主会话 + ${count - 1} 个子会话）` : "";
    stats.textContent = `共 ${count} 个会话${extra} · 内容体积合计 ${formatBytes(request.sizeBytes)}`;

    const rows: HTMLElement[] = [];
    request.sessionIds.forEach((id, index) => {
      const label = request.titles[index]?.trim() ?? "";
      const row = create("div", "chat-delete__item", [
        create("span", "", [
          label.length > 0 ? label : "未命名会话",
        ]),
        create("span", "mono muted", [id]),
      ]);
      row.setAttribute("role", "listitem");
      rows.push(row);
    });
    replaceChildren(list, rows);

    const reason = request.blockedReason;
    locked = reason !== null;
    blocked.hidden = !locked;
    blockedText.textContent = locked
      ? `已阻止删除：${reason}`
      : "";
    confirm.disabled = busy || locked;
    confirm.title = locked
      ? "检测到 OpenCode 正在运行，当前无法删除"
      : "删除会话并在删除前写入备份";
    showError("");
  }

  function closeWith(value: string): void {
    if (!busy && dialog.open) {
      dialog.close(value);
    }
  }

  close.addEventListener("click", () => {
    closeWith("cancel");
  });
  cancel.addEventListener("click", () => {
    closeWith("cancel");
  });
  confirm.addEventListener("click", () => {
    if (busy || locked || execute === null) {
      return;
    }
    const run = execute;
    showError("");
    setBusy(true);
    run()
      .then((result) => {
        setBusy(false);
        if (dialog.open) {
          dialog.close("deleted");
        }
        onDeleted(result);
      })
      .catch((error: unknown) => {
        setBusy(false);
        showError(errorMessage(error));
      });
  });
  dialog.addEventListener("cancel", (event) => {
    if (busy) {
      event.preventDefault();
    }
  });
  dialog.addEventListener("close", () => {
    execute = null;
    busy = false;
    locked = false;
    setBusy(false);
    showError("");
  });

  function open(
    request: ChatDeleteRequest,
    run: () => Promise<ChatDeleteResult>,
  ): void {
    if (dialog.open) {
      return;
    }
    execute = run;
    busy = false;
    renderPreview(request);
    dialog.returnValue = "";
    dialog.showModal();
    cancel.focus();
  }

  return { element: dialog, open };
}
