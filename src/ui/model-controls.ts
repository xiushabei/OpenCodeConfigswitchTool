import { icon } from "../icons/icons";
import type { CatalogueState } from "../models/catalogue";
import { create } from "./dom";

export function field(
  id: string,
  label: string,
  control: HTMLElement,
): HTMLDivElement {
  const text = create("label", "field__label", [label]);
  text.htmlFor = id;
  return create("div", "field", [text, control]);
}

export function selectWrap(select: HTMLSelectElement): HTMLDivElement {
  return create("div", "select-wrap", [select, icon("expand_more")]);
}

export function fillModelSelect(
  select: HTMLSelectElement,
  current: string | null,
  catalogue: CatalogueState,
  placeholder: string,
): void {
  const options: HTMLOptionElement[] = [];
  const empty = create("option", "", [placeholder]);
  empty.value = "";
  empty.disabled = true;
  empty.selected = current === null;
  options.push(empty);
  const known = new Set<string>();
  for (const entry of catalogue.models) {
    const option = create("option", "", [entry.id]);
    option.value = entry.id;
    option.selected = entry.id === current;
    options.push(option);
    known.add(entry.id);
  }
  if (current !== null && !known.has(current)) {
    const unknown = create("option", "", [`${current}（列表外）`]);
    unknown.value = current;
    unknown.selected = true;
    options.push(unknown);
  }
  select.replaceChildren(...options);
  if (current !== null) {
    select.value = current;
  }
}
