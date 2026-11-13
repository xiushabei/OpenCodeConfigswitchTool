import { icon } from "../icons/icons";
import type { Draft } from "../types";
import { TARGETS, isTarget } from "../types";
import { targetLabel } from "../format";
import { create } from "./dom";

export interface EditorFieldsView {
  element: HTMLElement;
  sync(draft: Draft, token: number): void;
  setBusy(busy: boolean): void;
  focusName(): void;
}

export interface EditorFieldsOptions {
  onChange(patch: Partial<Draft>): void;
}

interface FieldSpec {
  id: string;
  label: string;
  control: HTMLElement;
  className: string;
}

function labelledField(spec: FieldSpec): HTMLDivElement {
  const label = create("label", "field__label", [spec.label]);
  label.htmlFor = spec.id;
  const field = create("div", "field", [label, spec.control]);
  field.classList.add(spec.className);
  return field;
}

export function createEditorFields(
  options: EditorFieldsOptions,
): EditorFieldsView {
  const nameInput = create("input", "field__control");
  nameInput.type = "text";
  nameInput.id = "preset-name";
  nameInput.autocomplete = "off";
  nameInput.placeholder = "给预设一个名称";
  nameInput.addEventListener("input", () => {
    options.onChange({ name: nameInput.value });
  });

  const descriptionInput = create("input", "field__control");
  descriptionInput.type = "text";
  descriptionInput.id = "preset-description";
  descriptionInput.autocomplete = "off";
  descriptionInput.placeholder = "可选，说明这个预设的用途";
  descriptionInput.addEventListener("input", () => {
    options.onChange({ description: descriptionInput.value });
  });

  const targetSelect = create("select", "field__control");
  targetSelect.id = "preset-target";
  for (const target of TARGETS) {
    const option = create("option", "", [targetLabel(target)]);
    option.value = target;
    targetSelect.append(option);
  }
  targetSelect.addEventListener("change", () => {
    if (isTarget(targetSelect.value)) {
      options.onChange({ target: targetSelect.value });
    }
  });
  const selectWrap = create("div", "select-wrap", [
    targetSelect,
    icon("expand_more"),
  ]);

  const element = create("div", "editor__form", [
    labelledField({
      id: "preset-name",
      label: "名称",
      control: nameInput,
      className: "field--name",
    }),
    labelledField({
      id: "preset-target",
      label: "目标",
      control: selectWrap,
      className: "field--target",
    }),
    labelledField({
      id: "preset-description",
      label: "描述",
      control: descriptionInput,
      className: "field--wide",
    }),
  ]);

  let lastToken = -1;

  function sync(draft: Draft, token: number): void {
    if (token === lastToken) {
      return;
    }
    lastToken = token;
    nameInput.value = draft.name;
    descriptionInput.value = draft.description;
    targetSelect.value = draft.target;
  }

  function setBusy(busy: boolean): void {
    nameInput.readOnly = busy;
    descriptionInput.readOnly = busy;
    targetSelect.disabled = busy;
  }

  function focusName(): void {
    nameInput.focus();
    nameInput.select();
  }

  return { element, sync, setBusy, focusName };
}
