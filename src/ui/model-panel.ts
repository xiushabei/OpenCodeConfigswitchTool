import { patchOmoModelField } from "../actions";
import type { PatchOutcome } from "../actions/models";
import {
  ensureCatalogue,
  getCatalogue,
  subscribeCatalogue,
} from "../models/catalogue";
import type { CatalogueState } from "../models/catalogue";
import {
  lookupAssignment,
  readAssignments,
  validateAssignmentKey,
} from "../models/omo";
import { isOmoSection } from "../models/types";
import type { OmoSection } from "../models/types";
import { getState } from "../state/store";
import type { UiState } from "../state/store";
import type { Draft } from "../types";
import { button } from "./controls";
import { create, replaceChildren } from "./dom";
import { createModelAddForm } from "./model-add-form";
import { assignmentRow } from "./model-rows";

export interface ModelPanelView {
  element: HTMLElement;
  update(state: UiState, draft: Draft): void;
}

export interface ModelPanelOptions {
  onOpenJson(): void;
}

export function createModelPanel(
  options: ModelPanelOptions,
): ModelPanelView {
  const hint = create("p", "model-panel__hint", [
    "只修改 agents/categories 条目的 model 字段，其余字段保持不变。模型来自 OpenCode CLI 模型列表。",
  ]);

  const parseError = create("p", "model-panel__error", [""]);
  parseError.setAttribute("role", "alert");
  const openJson = button("打开 JSON 视图", "outlined", "code");
  openJson.addEventListener("click", options.onOpenJson);
  const errorBox = create("div", "model-panel__error-box", [
    parseError,
    openJson,
  ]);
  errorBox.hidden = true;

  const rowsTitle = create("h3", "model-panel__title", ["现有模型分配"]);
  const rows = create("div", "model-rows");
  const rowsEmpty = create("p", "model-panel__empty", [
    "尚未发现 agents 或 categories 中的 model 分配。",
  ]);
  rowsEmpty.hidden = true;
  const rowsSection = create("section", "model-panel__section", [
    rowsTitle,
    rows,
    rowsEmpty,
  ]);
  const addForm = createModelAddForm();
  const feedback = create("p", "model-panel__status", [""]);
  feedback.setAttribute("aria-live", "polite");

  const element = create("div", "model-panel", [
    hint,
    errorBox,
    rowsSection,
    addForm.element,
    feedback,
  ]);

  let lastState: UiState | null = null;
  let signature = "";
  let patching = false;
  let message = "";
  let focusedPath: string | null = null;

  rows.addEventListener("focusin", (event) => {
    const target = event.target;
    if (target instanceof HTMLElement) {
      focusedPath = target.dataset["path"] ?? focusedPath;
    }
  });

  function setMessage(text: string): void {
    message = text;
    feedback.textContent = text;
  }

  async function runPatch(
    section: OmoSection,
    key: string,
    model: string | null,
  ): Promise<boolean> {
    const state = getState();
    const draft = state.draft;
    if (
      patching ||
      state.busy !== null ||
      draft === null ||
      draft.target !== "omo"
    ) {
      return false;
    }
    patching = true;
    setMessage("正在更新配置…");
    if (lastState !== null) {
      render(lastState);
    }
    try {
      const outcome: PatchOutcome = await patchOmoModelField(
        section,
        key,
        model,
      );
      if (outcome === "applied") {
        setMessage(
          model === null
            ? `已清除 ${section}.${key}.model。`
            : `已更新 ${section}.${key}.model。`,
        );
        return true;
      }
      setMessage(
        outcome === "stale" ? "内容已变化，未应用本次更新。" : "",
      );
      return false;
    } finally {
      patching = false;
      if (lastState !== null) {
        render(lastState);
      }
    }
  }

  function syncCatalogue(
    catalogue: CatalogueState,
    disabled: boolean,
  ): void {
    addForm.sync(catalogue, disabled);
  }

  function render(state: UiState): void {
    lastState = state;
    const draft = state.draft;
    if (draft === null) {
      return;
    }
    const catalogue = getCatalogue();
    const parsed = readAssignments(draft.content);
    const busy = state.busy !== null || state.status !== "ready";
    const disabled = busy || patching;
    const next = JSON.stringify({
      content: draft.content,
      target: draft.target,
      tab: state.tab,
      busy: state.busy,
      patching,
      message,
      catalogue: [
        catalogue.status,
        catalogue.error,
        catalogue.directory,
        catalogue.models.map((entry) => entry.id),
      ],
    });
    if (next === signature) {
      return;
    }
    signature = next;
    const hadFocus = element.contains(document.activeElement);

    if (!parsed.ok) {
      errorBox.hidden = false;
      parseError.textContent = parsed.message;
      rowsSection.hidden = true;
      addForm.element.hidden = true;
    } else {
      errorBox.hidden = true;
      rowsSection.hidden = false;
      addForm.element.hidden = false;
      rowsEmpty.hidden = parsed.rows.length > 0;
      replaceChildren(
        rows,
        parsed.rows.map((row) =>
          assignmentRow(row, catalogue, disabled, (section, key, model) => {
            void runPatch(section, key, model);
          }),
        ),
      );
    }
    syncCatalogue(catalogue, disabled);
    setMessage(message);

    if (hadFocus && focusedPath !== null) {
      const match = Array.from(
        rows.querySelectorAll<HTMLElement>("[data-path]"),
      ).find((node) => node.dataset["path"] === focusedPath);
      match?.focus();
    }
  }

  subscribeCatalogue(() => {
    if (lastState !== null) {
      render(lastState);
    }
  });

  addForm.onSubmit((request) => {
    if (patching) {
      return;
    }
    const state = getState();
    const draft = state.draft;
    if (draft === null || draft.target !== "omo") {
      return;
    }
    const section = request.section;
    if (!isOmoSection(section)) {
      addForm.setError("请选择 agents 或 categories。");
      return;
    }
    const key = request.key.trim();
    const keyError = validateAssignmentKey(key);
    if (keyError !== null) {
      addForm.setError(keyError);
      return;
    }
    const model = request.model.trim();
    if (model.length === 0) {
      addForm.setError("请选择一个模型。");
      return;
    }
    const lookup = lookupAssignment(draft.content, section, key);
    if (!lookup.ok) {
      addForm.setError(lookup.message);
      return;
    }
    if (lookup.kind === "non-object") {
      addForm.setError(`${section}.${key} 已存在且不是对象，未修改。`);
      return;
    }
    if (lookup.kind === "has-model") {
      addForm.setError(`${section}.${key}.model 已存在，请在上方修改。`);
      return;
    }
    addForm.setError(null);
    void runPatch(section, key, model).then((applied) => {
      if (applied) {
        addForm.reset();
      }
    });
  });

  function update(state: UiState, draft: Draft): void {
    lastState = state;
    if (draft.target === "omo" && state.tab === "models") {
      ensureCatalogue();
      render(state);
    }
  }

  return { element, update };
}
