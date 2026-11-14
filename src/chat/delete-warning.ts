import { icon } from "../icons/icons";
import { button, iconButton } from "../ui/controls";
import { create } from "../ui/dom";
import {
  localDeleteWarningStore,
  type DeleteWarningStore,
} from "./delete-warning-store";
import "../styles/delete-warnings.css";

/** Exact first-use copy required by the product; do not reword. */
export const DELETE_WARNING_TEXT =
  "本功能目前处于测试阶段，常见的问题有：磁盘读写占满，电脑卡顿，整个删除过程大约5-8分钟，使用此功能产生的所有后果由用户承担";

export interface ChatDeleteWarningView {
  element: HTMLDialogElement;
  /**
   * Resolves true when the warning was already acknowledged on this local profile or the user
   * explicitly continues. Closing, Escape and cancel all resolve false without persisting.
   */
  ensure(): Promise<boolean>;
}

export function createChatDeleteWarning(
  store: DeleteWarningStore = localDeleteWarningStore(),
): ChatDeleteWarningView {
  const dialog = create("dialog", "dialog dialog--delete-warning");
  dialog.id = "chat-delete-warning";

  const title = create("h2", "dialog__title", ["删除功能测试提示"]);
  title.id = "chat-delete-warning-title";
  const warningIcon = icon("warning", 22);
  warningIcon.classList.add("delete-warning__icon");
  const close = iconButton("close", "取消并关闭提示");
  const head = create("div", "dialog__head", [warningIcon, title, close]);

  const text = create("p", "dialog__text", [DELETE_WARNING_TEXT]);
  text.id = "chat-delete-warning-text";
  const body = create("div", "dialog__body", [text]);

  const cancel = button("取消", "text");
  const proceed = button("我已知晓，继续", "filled");
  const actions = create("div", "dialog__actions", [cancel, proceed]);
  const form = create("div", "dialog__form", [head, body, actions]);
  dialog.append(form);
  dialog.setAttribute("aria-labelledby", "chat-delete-warning-title");
  dialog.setAttribute("aria-describedby", "chat-delete-warning-text");

  let pending: ((value: boolean) => void) | null = null;
  let opener: HTMLElement | null = null;

  function closeWith(value: "continue" | "cancel"): void {
    if (dialog.open) {
      dialog.close(value);
    }
  }

  function settle(): void {
    if (dialog.open) {
      // A queued close event from the previous show; the dialog has already been reopened.
      return;
    }
    const confirmed = dialog.returnValue === "continue";
    if (confirmed) {
      store.acknowledge();
    }
    if (pending !== null) {
      const resolve = pending;
      pending = null;
      resolve(confirmed);
    }
    if (opener !== null && opener.isConnected) {
      opener.focus();
    }
    opener = null;
  }

  close.addEventListener("click", () => {
    closeWith("cancel");
  });
  cancel.addEventListener("click", () => {
    closeWith("cancel");
  });
  proceed.addEventListener("click", () => {
    closeWith("continue");
  });
  dialog.addEventListener("close", settle);

  function ensure(): Promise<boolean> {
    if (store.acknowledged()) {
      return Promise.resolve(true);
    }
    if (dialog.open) {
      return Promise.resolve(false);
    }
    dialog.returnValue = "";
    opener =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    dialog.showModal();
    cancel.focus();
    return new Promise<boolean>((resolve) => {
      pending = resolve;
    });
  }

  return { element: dialog, ensure };
}
