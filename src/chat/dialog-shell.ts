import { button, iconButton } from "../ui/controls";
import { create } from "../ui/dom";

export interface ChatDialogShellOptions {
  list: HTMLElement;
  reader: HTMLElement;
  onDelete(): void;
  onClose(): void;
}

export interface ChatDialogShell {
  dialog: HTMLDialogElement;
  setDeleteEnabled(enabled: boolean, title: string): void;
}

export function createChatDialogShell(
  options: ChatDialogShellOptions,
): ChatDialogShell {
  const dialog = create("dialog", "dialog dialog--chat");
  dialog.id = "chat-dialog";

  const title = create("h2", "dialog__title", ["聊天记录"]);
  title.id = "chat-title";
  const close = iconButton("close", "关闭聊天记录");
  const head = create("div", "dialog__head", [title, close]);

  const deleteButton = button("删除所选会话", "text", "delete");
  deleteButton.classList.add("chat-delete-action");
  deleteButton.addEventListener("click", options.onDelete);
  const done = button("关闭", "tonal");
  done.addEventListener("click", options.onClose);
  const actions = create("div", "dialog__actions chat-actions", [
    deleteButton,
    done,
  ]);

  const layout = create("div", "chat-layout", [options.list, options.reader]);
  const body = create("div", "dialog__body chat-body", [layout]);
  const form = create("div", "dialog__form", [head, body, actions]);
  dialog.append(form);
  dialog.setAttribute("aria-labelledby", "chat-title");
  close.addEventListener("click", options.onClose);

  return {
    dialog,
    setDeleteEnabled: (enabled, label) => {
      deleteButton.disabled = !enabled;
      deleteButton.title = label;
    },
  };
}
