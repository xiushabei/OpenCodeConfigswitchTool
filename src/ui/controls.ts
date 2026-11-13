import { icon, type IconName } from "../icons/icons";
import { create } from "./dom";

export type ButtonKind = "filled" | "tonal" | "outlined" | "text";

export function button(
  label: string,
  kind: ButtonKind,
  iconName?: IconName,
): HTMLButtonElement {
  const node = create("button", `btn btn--${kind}`);
  node.type = "button";
  if (iconName !== undefined) {
    node.append(icon(iconName));
  }
  node.append(create("span", "btn__label", [label]));
  return node;
}

export function iconButton(
  name: IconName,
  label: string,
  danger: boolean = false,
): HTMLButtonElement {
  const node = create(
    "button",
    danger ? "icon-button icon-button--danger" : "icon-button",
  );
  node.type = "button";
  node.append(icon(name));
  node.setAttribute("aria-label", label);
  node.title = label;
  return node;
}

export function badge(
  text: string,
  variant: "active" | "dirty" | "error" | "target",
  iconName?: IconName,
): HTMLSpanElement {
  const node = create("span", `badge badge--${variant}`);
  if (iconName !== undefined) {
    node.append(icon(iconName, 14));
  }
  node.append(create("span", "", [text]));
  return node;
}

export function setButtonLabel(
  node: HTMLButtonElement,
  text: string,
): void {
  const label = node.querySelector(".btn__label");
  if (label !== null) {
    label.textContent = text;
  }
}
