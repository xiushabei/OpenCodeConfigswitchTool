import type { UiState } from "../state/store";
import {
  isDirty,
  targetPathFor,
  targetStateFor,
} from "../state/selectors";
import type { Draft } from "../types";
import { targetLabel } from "../format";
import { badge } from "./controls";
import { create } from "./dom";

export interface EditorHeaderView {
  element: HTMLElement;
  update(state: UiState, draft: Draft): void;
}

export function createEditorHeader(): EditorHeaderView {
  const title = create("h2", "editor__title", ["编辑预设"]);
  title.id = "editor-title";
  const activeBadge = badge("当前生效", "active", "check_circle");
  activeBadge.hidden = true;
  const dirtyBadge = badge("未保存", "dirty");
  dirtyBadge.hidden = true;
  const titleRow = create("div", "editor__title-row", [
    title,
    activeBadge,
    dirtyBadge,
  ]);

  const targetName = create("strong", "", ["OpenCode"]);
  const targetPath = create("span", "editor__target-path mono", ["—"]);
  const targetProblem = badge("目标读取失败", "error", "error");
  targetProblem.hidden = true;
  const targetLine = create("p", "editor__target-line", [
    create("span", "", ["目标："]),
    targetName,
    targetPath,
    targetProblem,
  ]);

  const heading = create("div", "editor__heading", [titleRow, targetLine]);
  const element = create("header", "editor__head", [heading]);

  function update(state: UiState, draft: Draft): void {
    const isNew = draft.id.length === 0;
    title.textContent = isNew ? "新建预设" : "编辑预设";
    const target = targetStateFor(state.snapshot, draft.target);
    const active =
      !isNew && (target?.activePresetIds.includes(draft.id) ?? false);
    activeBadge.hidden = !active;
    dirtyBadge.hidden = !isDirty(state);
    targetName.textContent = targetLabel(draft.target);
    targetPath.textContent = target?.path ?? targetPathFor(state, draft.target);
    targetProblem.hidden = target === null || target.error === null;
    targetProblem.title = target?.error ?? "";
  }

  return { element, update };
}
