import type { CatalogueState } from "../models/catalogue";
import type { ModelAssignment, OmoSection } from "../models/types";
import { button } from "./controls";
import { create } from "./dom";
import { fillModelSelect, selectWrap } from "./model-controls";

export type PatchAssignment = (
  section: OmoSection,
  key: string,
  model: string | null,
) => void;

export function assignmentRow(
  row: ModelAssignment,
  catalogue: CatalogueState,
  disabled: boolean,
  onPatch: PatchAssignment,
): HTMLElement {
  const path = `${row.section}.${row.key}`;
  const select = create("select", "field__control");
  select.setAttribute("aria-label", `${path}.model`);
  select.dataset["path"] = path;
  fillModelSelect(select, row.model, catalogue, "未设置");
  select.disabled = disabled;
  select.addEventListener("change", () => {
    const value = select.value.trim();
    if (value.length === 0 || value === row.model) {
      return;
    }
    onPatch(row.section, row.key, value);
  });

  const clear = button("清除", "text");
  clear.dataset["path"] = path;
  clear.disabled = disabled || row.model === null;
  clear.title =
    row.model === null ? "没有 model 覆盖" : `清除 ${path}.model`;
  clear.addEventListener("click", () => {
    onPatch(row.section, row.key, null);
  });

  return create("div", "model-row", [
    create("div", "model-row__label", [
      create("span", "model-row__key", [row.key]),
      create("span", "model-row__path mono", [`${path}.model`]),
    ]),
    create("div", "model-row__controls", [selectWrap(select), clear]),
  ]);
}
