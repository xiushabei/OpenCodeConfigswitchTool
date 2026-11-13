import type { CatalogueState } from "../models/catalogue";
import { loadCatalogue } from "../models/catalogue";
import { OMO_SECTIONS } from "../models/types";
import { button } from "./controls";
import { create } from "./dom";
import { field, fillModelSelect, selectWrap } from "./model-controls";

export interface ModelAddRequest {
  readonly section: string;
  readonly key: string;
  readonly model: string;
}

export interface ModelAddFormView {
  element: HTMLElement;
  onSubmit(handler: (request: ModelAddRequest) => void): void;
  sync(catalogue: CatalogueState, disabled: boolean): void;
  setError(message: string | null): void;
  reset(): void;
}

export function createModelAddForm(): ModelAddFormView {
  const sectionSelect = create("select", "field__control");
  sectionSelect.id = "model-add-section";
  for (const section of OMO_SECTIONS) {
    const option = create("option", "", [section]);
    option.value = section;
    sectionSelect.append(option);
  }

  const keyInput = create("input", "field__control");
  keyInput.type = "text";
  keyInput.id = "model-add-key";
  keyInput.autocomplete = "off";
  keyInput.spellcheck = false;
  keyInput.placeholder = "例如 build";

  const modelSelect = create("select", "field__control");
  modelSelect.id = "model-add-model";

  const addButton = button("添加分配", "tonal", "add");
  addButton.type = "submit";

  const form = create("form", "model-add");
  form.append(
    field("model-add-section", "分区", selectWrap(sectionSelect)),
    field("model-add-key", "键名", keyInput),
    field("model-add-model", "模型", selectWrap(modelSelect)),
    addButton,
  );

  const note = create("p", "model-panel__note", [""]);
  const retry = button("重新读取", "text");
  retry.hidden = true;
  retry.addEventListener("click", () => {
    void loadCatalogue(false);
  });

  const error = create("p", "model-panel__error", [""]);
  error.setAttribute("role", "alert");
  error.hidden = true;

  const body = create("div", "model-panel__add-body", [
    create("p", "model-panel__hint", [
      "键名由你填写，不会自动创建 agent 或 category 配置；只写入所选模型的 model 字段。",
    ]),
    form,
    note,
    retry,
    error,
  ]);
  const element = create("section", "model-panel__section", [
    create("h3", "model-panel__title", ["新增分配"]),
    body,
  ]);

  let catalogueSnapshot: CatalogueState | null = null;
  let disabled = false;
  let handler: ((request: ModelAddRequest) => void) | null = null;

  form.addEventListener("submit", (event) => {
    event.preventDefault();
    if (disabled || handler === null) {
      return;
    }
    handler({
      section: sectionSelect.value,
      key: keyInput.value,
      model: modelSelect.value,
    });
  });

  function setError(message: string | null): void {
    error.textContent = message ?? "";
    error.hidden = message === null;
  }

  function sync(catalogue: CatalogueState, nextDisabled: boolean): void {
    catalogueSnapshot = catalogue;
    disabled = nextDisabled;
    if (catalogue.status === "loading") {
      note.textContent = "正在读取模型列表…";
      note.setAttribute("data-tone", "neutral");
      retry.hidden = true;
    } else if (catalogue.status === "error") {
      note.textContent = `模型列表读取失败：${catalogue.error ?? ""}`;
      note.setAttribute("data-tone", "error");
      retry.hidden = false;
    } else if (catalogue.models.length === 0) {
      note.textContent = "模型列表为空；请在上方“模型列表”中刷新。";
      note.setAttribute("data-tone", "neutral");
      retry.hidden = true;
    } else {
      note.textContent = `模型列表：${catalogue.models.length} 个可选模型。`;
      note.setAttribute("data-tone", "neutral");
      retry.hidden = true;
    }
    retry.disabled = disabled;
    const selected = modelSelect.value.trim();
    fillModelSelect(
      modelSelect,
      selected.length > 0 ? selected : null,
      catalogue,
      "选择模型",
    );
    sectionSelect.disabled = disabled;
    keyInput.readOnly = disabled;
    modelSelect.disabled = disabled;
    addButton.disabled = disabled;
  }

  function reset(): void {
    keyInput.value = "";
    if (catalogueSnapshot !== null) {
      fillModelSelect(modelSelect, null, catalogueSnapshot, "选择模型");
    }
  }

  return {
    element,
    onSubmit: (next) => {
      handler = next;
    },
    sync,
    setError,
    reset,
  };
}
