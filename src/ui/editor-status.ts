import { icon, type IconName } from "../icons/icons";
import type { UiState } from "../state/store";
import type { Draft } from "../types";
import { splitLines } from "../diff";
import { create, replaceChildren } from "./dom";

export interface EditorStatusView {
  element: HTMLElement;
  update(state: UiState, draft: Draft): void;
}

interface StatusContent {
  tone: "neutral" | "success" | "error";
  iconName: IconName;
  text: string;
}

function contentFor(state: UiState, draft: Draft): StatusContent {
  if (state.jsonError !== null) {
    return {
      tone: "error",
      iconName: "error",
      text: state.jsonError,
    };
  }
  if (draft.content.trim().length === 0) {
    return {
      tone: "neutral",
      iconName: "info",
      text: "等待输入 JSON 对象。",
    };
  }
  return {
    tone: "success",
    iconName: "check_circle",
    text: "JSON 有效，可以保存。",
  };
}

export function createEditorStatus(): EditorStatusView {
  const statusIcon = create("span", "editor__status-icon", []);
  const statusText = create("span", "", [""]);
  const statusMain = create("span", "editor__status-main", [
    statusIcon,
    statusText,
  ]);
  statusMain.id = "json-status";
  statusMain.setAttribute("aria-live", "polite");
  const lineCount = create("span", "", ["0 行"]);
  const shortcut = create("span", "", ["Ctrl+S 保存"]);
  const statusMeta = create("div", "editor__status-meta", [
    lineCount,
    shortcut,
  ]);
  const element = create("footer", "editor__status", [
    statusMain,
    statusMeta,
  ]);

  let lastSignature = "";

  function update(state: UiState, draft: Draft): void {
    const content = contentFor(state, draft);
    const next = `${content.tone}\u0000${content.iconName}\u0000${content.text}`;
    if (next !== lastSignature) {
      lastSignature = next;
      replaceChildren(statusIcon, [icon(content.iconName, 18)]);
      statusText.textContent = content.text;
      statusMain.setAttribute("data-tone", content.tone);
    }
    lineCount.textContent = `${splitLines(draft.content).length} 行`;
  }

  return { element, update };
}
