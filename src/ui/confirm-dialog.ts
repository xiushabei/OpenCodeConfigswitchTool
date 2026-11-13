import { button, iconButton, setButtonLabel } from "./controls";
import { create } from "./dom";

export interface ConfirmOptions {
  title: string;
  message: string;
  detail: string | null;
  confirmLabel: string;
  danger: boolean;
}

export interface ConfirmDialogView {
  element: HTMLDialogElement;
  ask(options: ConfirmOptions): Promise<boolean>;
}

export function createConfirmDialog(): ConfirmDialogView {
  const dialog = create("dialog", "dialog");
  dialog.id = "confirm-dialog";

  const title = create("h2", "dialog__title", ["确认操作"]);
  title.id = "confirm-title";
  const close = iconButton("close", "取消并关闭");
  const head = create("div", "dialog__head", [title, close]);

  const message = create("p", "dialog__text", [""]);
  const detail = create("p", "dialog__note mono", [""]);
  detail.hidden = true;
  const body = create("div", "dialog__body", [message, detail]);

  const cancel = button("取消", "text");
  const confirm = button("确认", "filled");
  const actions = create("div", "dialog__actions", [cancel, confirm]);

  const form = create("div", "dialog__form", [head, body, actions]);
  dialog.append(form);
  dialog.setAttribute("aria-labelledby", "confirm-title");

  let resolver: ((value: boolean) => void) | null = null;
  let opener: HTMLElement | null = null;

  function settle(value: boolean): void {
    if (resolver !== null) {
      const resolve = resolver;
      resolver = null;
      resolve(value);
    }
  }

  function closeWith(result: "confirm" | "cancel"): void {
    if (dialog.open) {
      dialog.close(result);
    }
  }

  close.addEventListener("click", () => {
    closeWith("cancel");
  });
  cancel.addEventListener("click", () => {
    closeWith("cancel");
  });
  confirm.addEventListener("click", () => {
    closeWith("confirm");
  });
  dialog.addEventListener("close", () => {
    settle(dialog.returnValue === "confirm");
    if (opener !== null) {
      opener.focus();
    }
    opener = null;
  });

  function ask(options: ConfirmOptions): Promise<boolean> {
    if (dialog.open) {
      return Promise.resolve(false);
    }
    title.textContent = options.title;
    message.textContent = options.message;
    detail.textContent = options.detail ?? "";
    detail.hidden = options.detail === null;
    setButtonLabel(confirm, options.confirmLabel);
    confirm.classList.toggle("btn--critical", options.danger);
    dialog.returnValue = "";
    opener =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    dialog.showModal();
    cancel.focus();
    return new Promise<boolean>((resolve) => {
      resolver = resolve;
    });
  }

  return { element: dialog, ask };
}
