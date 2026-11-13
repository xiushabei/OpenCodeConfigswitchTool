import { ensureCatalogueReady, getCatalogue } from "../models/catalogue";
import { button, iconButton } from "./controls";
import { create } from "./dom";

export interface ModelTabGate {
  element: HTMLDialogElement;
  request(opener: HTMLElement | null, onEnter: () => void): void;
}

interface ActiveRequest {
  readonly onEnter: () => void;
}

export function createModelTabGate(): ModelTabGate {
  const dialog = create("dialog", "dialog dialog--model-gate");
  dialog.id = "model-gate-dialog";

  const title = create("h2", "dialog__title", ["正在加载模型列表"]);
  title.id = "model-gate-title";
  const close = iconButton("close", "取消加载");
  const head = create("div", "dialog__head", [title, close]);

  const description = create("p", "dialog__text", [
    "模型选择需要 OpenCode CLI 的模型列表。读取完成后会自动切换到模型选择。",
  ]);
  const statusText = create("p", "model-gate__status", [""]);
  statusText.setAttribute("role", "status");
  statusText.setAttribute("aria-live", "polite");
  statusText.setAttribute("aria-atomic", "true");

  const track = create("div", "model-gate__track");
  track.setAttribute("aria-hidden", "true");
  track.append(create("div", "model-gate__bar"));
  const loadingState = create("div", "model-gate__state", [
    description,
    statusText,
    track,
  ]);

  const errorLead = create("p", "dialog__text", [
    "模型列表读取失败，未切换到模型选择。",
  ]);
  const reason = create("p", "dialog__error", [""]);
  reason.setAttribute("role", "alert");
  const errorState = create("div", "model-gate__state", [errorLead, reason]);
  errorState.hidden = true;

  const body = create("div", "dialog__body", [loadingState, errorState]);

  const cancel = button("取消", "text");
  const retry = button("重试", "outlined", "refresh");
  retry.hidden = true;
  const actions = create("div", "dialog__actions", [cancel, retry]);

  const form = create("div", "dialog__form", [head, body, actions]);
  dialog.append(form);
  dialog.setAttribute("aria-labelledby", "model-gate-title");

  let active: ActiveRequest | null = null;
  let opener: HTMLElement | null = null;

  function showLoading(): void {
    title.textContent = "正在加载模型列表";
    loadingState.hidden = false;
    errorState.hidden = true;
    retry.hidden = true;
    retry.disabled = false;
    close.setAttribute("aria-label", "取消加载");
  }

  function announceLoading(retrying: boolean): void {
    statusText.textContent = retrying
      ? "正在重新读取模型列表…"
      : "正在读取模型列表…";
  }

  function showError(message: string): void {
    title.textContent = "无法加载模型列表";
    loadingState.hidden = true;
    errorState.hidden = false;
    reason.textContent = message;
    retry.hidden = false;
    close.setAttribute("aria-label", "取消并关闭");
  }

  function complete(request: ActiveRequest): void {
    active = null;
    opener = null;
    if (dialog.open) {
      dialog.close("ready");
    }
    request.onEnter();
  }

  async function finish(request: ActiveRequest): Promise<void> {
    await ensureCatalogueReady();
    if (active !== request) {
      return;
    }
    const catalogue = getCatalogue();
    if (catalogue.status === "ready") {
      complete(request);
      return;
    }
    showError(catalogue.error ?? "未知错误，请重试。");
  }

  function request(
    openerElement: HTMLElement | null,
    onEnter: () => void,
  ): void {
    if (active !== null) {
      return;
    }
    if (getCatalogue().status === "ready") {
      onEnter();
      return;
    }
    const next: ActiveRequest = { onEnter };
    active = next;
    opener = openerElement;
    dialog.returnValue = "";
    showLoading();
    statusText.textContent = "";
    if (!dialog.open) {
      dialog.showModal();
    }
    cancel.focus();
    announceLoading(false);
    void finish(next);
  }

  function cancelRequest(): void {
    active = null;
    if (dialog.open) {
      dialog.close("cancel");
    }
  }

  close.addEventListener("click", cancelRequest);
  cancel.addEventListener("click", cancelRequest);
  retry.addEventListener("click", () => {
    const current = active;
    if (current === null) {
      return;
    }
    showLoading();
    cancel.focus();
    announceLoading(true);
    void finish(current);
  });

  dialog.addEventListener("cancel", () => {
    active = null;
  });
  dialog.addEventListener("close", () => {
    active = null;
    if (opener !== null) {
      opener.focus();
    }
    opener = null;
  });

  return { element: dialog, request };
}
