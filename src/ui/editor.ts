import { icon } from "../icons/icons";
import type { EditorTab, UiState } from "../state/store";
import { create } from "./dom";
import { createDiffView } from "./diff-view";
import { createEditorActions } from "./editor-actions";
import { createEditorEmpty } from "./editor-empty";
import { createEditorFields } from "./editor-fields";
import { createEditorHeader } from "./editor-header";
import { createEditorStatus } from "./editor-status";
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

  const tabEdit = create("button", "tab", [
    icon("code", 18),
    create("span", "", ["编辑"]),
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

  const tabs = create("div", "tabs", [tabEdit, tabDiff]);
  tabs.setAttribute("role", "tablist");
  tabs.setAttribute("aria-label", "编辑视图");

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
    const next: EditorTab =
      event.key === "ArrowRight" || event.key === "End" ? "diff" : "edit";
    handlers.onTabChange(next);
    const button = next === "edit" ? tabEdit : tabDiff;
    button.focus();
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

  const toolbarRow = create("div", "editor__toolbar-row", [
    actionsView.element,
    tabs,
  ]);
  const body = create("div", "editor__body", [editPanel, diffPanel]);

  const editorElement = create("div", "editor", [
    headerView.element,
    fieldsView.element,
    toolbarRow,
    body,
    statusView.element,
  ]);

  const emptyView = createEditorEmpty(handlers);
  const element = create("section", "main", [
    editorElement,
    emptyView.element,
  ]);
  element.setAttribute("aria-label", "预设工作区");

  let lastToken = -1;

  function syncTabs(state: UiState): void {
    const editSelected = state.tab === "edit";
    tabEdit.setAttribute("aria-selected", String(editSelected));
    tabDiff.setAttribute("aria-selected", String(!editSelected));
    tabEdit.tabIndex = editSelected ? 0 : -1;
    tabDiff.tabIndex = editSelected ? -1 : 0;
    editPanel.hidden = !editSelected;
    diffPanel.hidden = editSelected;
    if (!editSelected) {
      diffView.update(state);
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
    syncTabs(state);
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
