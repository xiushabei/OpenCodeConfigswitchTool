import { icon } from "../icons/icons";
import { getState } from "../state/store";
import type { EditorTab, UiState } from "../state/store";
import type { Draft } from "../types";
import { create } from "./dom";
import { createDiffView } from "./diff-view";
import { createEditorActions } from "./editor-actions";
import { createEditorEmpty } from "./editor-empty";
import { createEditorFields } from "./editor-fields";
import { createEditorHeader } from "./editor-header";
import { createEditorStatus } from "./editor-status";
import { createModelPanel } from "./model-panel";
import { createModelTabGate } from "./model-tab-gate";
import type { AppHandlers } from "./handlers";

export interface EditorView {
  element: HTMLElement;
  update(state: UiState): void;
  focusName(): void;
}

export function createEditor(handlers: AppHandlers): EditorView {
  const headerView = createEditorHeader();
  const fieldsView = createEditorFields({ onChange: handlers.onDraftChange });
  const actionsView = createEditorActions(handlers);
  const statusView = createEditorStatus();
  const modelPanel = createModelPanel({
    onOpenJson: () => {
      handlers.onTabChange("edit");
    },
  });
  const modelGate = createModelTabGate();

  const tabModels = create("button", "tab", [
    icon("sync_alt", 18),
    create("span", "", ["模型选择"]),
  ]);
  tabModels.type = "button";
  tabModels.id = "tab-models";
  tabModels.setAttribute("role", "tab");
  tabModels.setAttribute("aria-controls", "panel-models");
  tabModels.addEventListener("click", () => {
    requestModelsTab(tabModels);
  });

  const tabEdit = create("button", "tab", [
    icon("code", 18),
    create("span", "", ["JSON"]),
  ]);
  tabEdit.type = "button";
  tabEdit.id = "tab-edit";
  tabEdit.setAttribute("role", "tab");
  tabEdit.setAttribute("aria-controls", "panel-edit");
  tabEdit.addEventListener("click", () => {
    handlers.onTabChange("edit");
  });

  const tabDiff = create("button", "tab", [
    icon("difference", 18),
    create("span", "", ["差异"]),
  ]);
  tabDiff.type = "button";
  tabDiff.id = "tab-diff";
  tabDiff.setAttribute("role", "tab");
  tabDiff.setAttribute("aria-controls", "panel-diff");
  tabDiff.addEventListener("click", () => {
    handlers.onTabChange("diff");
  });

  const tabs = create("div", "tabs", [tabModels, tabEdit, tabDiff]);
  tabs.setAttribute("role", "tablist");
  tabs.setAttribute("aria-label", "编辑视图");

  const tabButtons: Record<EditorTab, HTMLButtonElement> = {
    models: tabModels,
    edit: tabEdit,
    diff: tabDiff,
  };
  let currentTab: EditorTab = "edit";
  let hasOmoTarget = false;

  function requestModelsTab(opener: HTMLElement | null): void {
    const state = getState();
    if (
      state.tab === "models" ||
      state.draft === null ||
      state.draft.target !== "omo"
    ) {
      return;
    }
    const token = state.selectionToken;
    modelGate.request(opener, () => {
      const next = getState();
      if (
        next.draft === null ||
        next.draft.target !== "omo" ||
        next.selectionToken !== token
      ) {
        return;
      }
      handlers.onTabChange("models");
      tabButtons["models"].focus();
    });
  }

  tabs.addEventListener("keydown", (event: KeyboardEvent) => {
    if (
      event.key !== "ArrowLeft" &&
      event.key !== "ArrowRight" &&
      event.key !== "Home" &&
      event.key !== "End"
    ) {
      return;
    }
    event.preventDefault();
    const order: readonly EditorTab[] = hasOmoTarget
      ? ["models", "edit", "diff"]
      : ["edit", "diff"];
    const index = Math.max(0, order.indexOf(currentTab));
    const nextIndex =
      event.key === "Home"
        ? 0
        : event.key === "End"
          ? order.length - 1
          : event.key === "ArrowRight"
            ? (index + 1) % order.length
            : (index - 1 + order.length) % order.length;
    const next = order[nextIndex] ?? "edit";
    if (next === "models") {
      requestModelsTab(tabButtons[currentTab] ?? tabModels);
      return;
    }
    handlers.onTabChange(next);
    tabButtons[next].focus();
  });

  const contentLabel = create("label", "field__label", ["JSON 内容"]);
  contentLabel.htmlFor = "preset-content";
  const textarea = create("textarea", "json-editor");
  textarea.id = "preset-content";
  textarea.spellcheck = false;
  textarea.rows = 12;
  textarea.setAttribute("wrap", "off");
  textarea.setAttribute("aria-describedby", "json-status");
  textarea.placeholder = "在此粘贴或编辑 JSON 对象";
  textarea.addEventListener("input", () => {
    handlers.onDraftChange({ content: textarea.value });
  });

  const editPanel = create("div", "editor__panel", [
    contentLabel,
    textarea,
  ]);
  editPanel.id = "panel-edit";
  editPanel.setAttribute("role", "tabpanel");
  editPanel.setAttribute("aria-labelledby", "tab-edit");

  const diffView = createDiffView();
  const diffPanel = create("div", "editor__panel", [diffView.element]);
  diffPanel.id = "panel-diff";
  diffPanel.setAttribute("role", "tabpanel");
  diffPanel.setAttribute("aria-labelledby", "tab-diff");

  const modelsPanel = create("div", "editor__panel", [modelPanel.element]);
  modelsPanel.id = "panel-models";
  modelsPanel.setAttribute("role", "tabpanel");
  modelsPanel.setAttribute("aria-labelledby", "tab-models");

  const toolbarRow = create("div", "editor__toolbar-row", [
    actionsView.element,
    tabs,
  ]);
  const body = create("div", "editor__body", [
    modelsPanel,
    editPanel,
    diffPanel,
  ]);

  const editorElement = create("div", "editor", [
    headerView.element,
    fieldsView.element,
    toolbarRow,
    body,
    statusView.element,
    modelGate.element,
  ]);

  const emptyView = createEditorEmpty(handlers);
  const element = create("section", "main", [
    editorElement,
    emptyView.element,
  ]);
  element.setAttribute("aria-label", "预设工作区");

  let lastToken = -1;

  function syncTabs(state: UiState, draft: Draft): void {
    const omo = draft.target === "omo";
    hasOmoTarget = omo;
    tabModels.hidden = !omo;
    const selected: EditorTab =
      state.tab === "models" && !omo ? "edit" : state.tab;
    currentTab = selected;
    const editSelected = selected === "edit";
    const diffSelected = selected === "diff";
    const modelsSelected = selected === "models";
    tabModels.setAttribute("aria-selected", String(modelsSelected));
    tabEdit.setAttribute("aria-selected", String(editSelected));
    tabDiff.setAttribute("aria-selected", String(diffSelected));
    tabModels.tabIndex = modelsSelected ? 0 : -1;
    tabEdit.tabIndex = editSelected ? 0 : -1;
    tabDiff.tabIndex = diffSelected ? 0 : -1;
    modelsPanel.hidden = !modelsSelected;
    editPanel.hidden = !editSelected;
    diffPanel.hidden = !diffSelected;
    if (diffSelected) {
      diffView.update(state);
    }
    if (modelsSelected) {
      modelPanel.update(state, draft);
    }
  }

  function update(state: UiState): void {
    const draft = state.draft;
    const showEditor = state.status === "ready" && draft !== null;
    editorElement.hidden = !showEditor;
    emptyView.element.hidden = showEditor;

    if (!showEditor) {
      emptyView.update(state);
      lastToken = -1;
      return;
    }

    const busy = state.busy !== null;
    fieldsView.setBusy(busy || state.status !== "ready");
    textarea.readOnly = busy;
    if (state.selectionToken !== lastToken) {
      fieldsView.sync(draft, state.selectionToken);
      textarea.value = draft.content;
      textarea.scrollTop = 0;
      lastToken = state.selectionToken;
    }
    headerView.update(state, draft);
    syncTabs(state, draft);
    statusView.update(state, draft);
    textarea.setAttribute("aria-invalid", String(state.jsonError !== null));
    editorElement.setAttribute("aria-busy", String(busy));
    actionsView.update(state);
  }

  function focusName(): void {
    if (editorElement.hidden) {
      return;
    }
    fieldsView.focusName();
  }

  return { element, update, focusName };
}
