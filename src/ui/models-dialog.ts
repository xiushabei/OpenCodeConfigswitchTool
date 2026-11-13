import { icon } from "../icons/icons";
import {
  ensureCatalogue,
  getCatalogue,
  loadCatalogue,
  subscribeCatalogue,
} from "../models/catalogue";
import type { CatalogueState } from "../models/catalogue";
import type { ModelOption } from "../models/types";
import { badge, button, iconButton } from "./controls";
import { create, replaceChildren } from "./dom";

export interface ModelsDialogView {
  element: HTMLDialogElement;
  open(): void;
}

function uniqueProviders(models: readonly ModelOption[]): string[] {
  const providers = new Set<string>();
  for (const entry of models) {
    providers.add(entry.provider);
  }
  return [...providers].sort((left, right) => left.localeCompare(right));
}

function matches(entry: ModelOption, query: string, provider: string): boolean {
  if (provider !== "all" && entry.provider !== provider) {
    return false;
  }
  if (query.length === 0) {
    return true;
  }
  return (
    entry.id.toLowerCase().includes(query) ||
    entry.provider.toLowerCase().includes(query) ||
    entry.model.toLowerCase().includes(query)
  );
}

export function createModelsDialog(): ModelsDialogView {
  const dialog = create("dialog", "dialog dialog--models");
  dialog.id = "models-dialog";

  const title = create("h2", "dialog__title", ["模型列表"]);
  title.id = "models-title";
  const close = iconButton("close", "关闭模型列表");
  const head = create("div", "dialog__head", [title, close]);

  const scopeText = create("span", "", [
    "来源：已安装的 OpenCode CLI 模型命令，合并当前项目配置与继承的凭据、插件。",
  ]);
  const scopePath = create("span", "mono model-scope__path", ["—"]);
  const scope = create("p", "dialog__note", [scopeText, scopePath]);

  const filterInput = create("input", "field__control");
  filterInput.type = "search";
  filterInput.id = "models-filter";
  filterInput.autocomplete = "off";
  filterInput.spellcheck = false;
  filterInput.placeholder = "按 provider 或 model 过滤";
  const filterLabel = create("label", "field__label", ["过滤"]);
  filterLabel.htmlFor = "models-filter";
  const filterField = create("div", "field", [filterLabel, filterInput]);

  const providerSelect = create("select", "field__control");
  providerSelect.id = "models-provider";
  const providerLabel = create("label", "field__label", ["Provider"]);
  providerLabel.htmlFor = "models-provider";
  const providerWrap = create("div", "select-wrap", [
    providerSelect,
    icon("expand_more"),
  ]);
  const providerField = create("div", "field", [
    providerLabel,
    providerWrap,
  ]);

  const refresh = button("刷新列表", "outlined", "refresh");
  const tools = create("div", "model-tools", [
    filterField,
    providerField,
    refresh,
  ]);

  const status = create("p", "model-status", [""]);
  status.setAttribute("role", "status");
  status.setAttribute("aria-live", "polite");

  const list = create("div", "model-list");
  list.setAttribute("role", "list");
  list.setAttribute("aria-label", "可用模型");

  const body = create("div", "dialog__body", [scope, tools, status, list]);
  const done = button("关闭", "text");
  const actions = create("div", "dialog__actions", [done]);
  const form = create("div", "dialog__form", [head, body, actions]);
  dialog.append(form);
  dialog.setAttribute("aria-labelledby", "models-title");

  let query = "";
  let provider = "all";
  let opener: HTMLElement | null = null;
  let providerSignature: string | null = null;

  function syncProviders(state: CatalogueState): void {
    const providers = uniqueProviders(state.models);
    if (provider !== "all" && !providers.includes(provider)) {
      provider = "all";
    }
    const nextSignature = providers.join("\u0000");
    if (nextSignature !== providerSignature) {
      providerSignature = nextSignature;
      const options: HTMLOptionElement[] = [];
      const all = create("option", "", ["全部 provider"]);
      all.value = "all";
      options.push(all);
      for (const name of providers) {
        const option = create("option", "", [name]);
        option.value = name;
        options.push(option);
      }
      replaceChildren(providerSelect, options);
    }
    providerSelect.value = provider;
  }

  function row(entry: ModelOption): HTMLElement {
    const item = create("div", "model-entry", [
      create("span", "model-entry__id mono", [entry.id]),
      badge(entry.provider, "target"),
    ]);
    item.setAttribute("role", "listitem");
    item.title = `${entry.provider} / ${entry.model}`;
    return item;
  }

  function renderPlaceholder(text: string): void {
    replaceChildren(list, [create("p", "model-list__empty", [text])]);
  }

  function render(): void {
    const state = getCatalogue();
    syncProviders(state);
    scopePath.textContent =
      state.directory.length > 0 ? `运行目录：${state.directory}` : "运行目录：—";

    const loading = state.status === "loading";
    refresh.disabled = loading;
    filterInput.disabled = false;
    providerSelect.disabled = false;

    const normalized = query.trim().toLowerCase();
    const visible = state.models.filter((entry) =>
      matches(entry, normalized, provider),
    );

    if (state.status === "error" && state.error !== null) {
      status.textContent = `读取模型列表失败：${state.error}`;
      status.setAttribute("data-tone", "error");
    } else if (loading) {
      status.textContent =
        state.models.length === 0
          ? "正在读取模型列表…"
          : "正在刷新模型列表…";
      status.setAttribute("data-tone", "neutral");
    } else if (state.models.length === 0) {
      status.textContent = "CLI 未返回任何模型。";
      status.setAttribute("data-tone", "neutral");
    } else {
      const counts =
        visible.length === state.models.length
          ? `共 ${state.models.length} 个模型`
          : `显示 ${visible.length} / 共 ${state.models.length} 个模型`;
      status.textContent = counts;
      status.setAttribute("data-tone", "neutral");
    }

    if (state.models.length === 0) {
      const message =
        state.status === "loading"
          ? "正在读取…"
          : state.status === "error"
            ? "读取失败，可点击“刷新列表”重试。"
            : "请确认 OpenCode CLI 已安装并登录，然后刷新。";
      renderPlaceholder(message);
      return;
    }
    if (visible.length === 0) {
      renderPlaceholder("没有匹配的模型。");
      return;
    }
    replaceChildren(list, visible.map((entry) => row(entry)));
  }

  close.addEventListener("click", () => {
    if (dialog.open) {
      dialog.close("close");
    }
  });
  done.addEventListener("click", () => {
    if (dialog.open) {
      dialog.close("close");
    }
  });
  dialog.addEventListener("close", () => {
    if (opener !== null) {
      opener.focus();
    }
    opener = null;
  });
  filterInput.addEventListener("input", () => {
    query = filterInput.value;
    render();
  });
  providerSelect.addEventListener("change", () => {
    provider = providerSelect.value;
    render();
  });
  refresh.addEventListener("click", () => {
    void loadCatalogue(true);
  });

  subscribeCatalogue(render);

  function open(): void {
    opener =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    dialog.returnValue = "";
    render();
    if (!dialog.open) {
      dialog.showModal();
    }
    filterInput.focus();
    filterInput.select();
    ensureCatalogue();
  }

  return { element: dialog, open };
}
