import { diffLines, MAX_RENDER_LINES } from "../diff";
import type { DiffLine, DiffResult } from "../diff";
import { targetLabel } from "../format";
import type { UiState } from "../state/store";
import { targetPathFor, targetStateFor } from "../state/selectors";
import { create, replaceChildren } from "./dom";

export interface DiffView {
  element: HTMLElement;
  update(state: UiState): void;
}

function diffRow(line: DiffLine): HTMLDivElement {
  const row = create("div", "diff__row");
  row.setAttribute("data-kind", line.kind);
  const number = line.kind === "add" ? line.newLine : line.oldLine;
  const sign = line.kind === "add" ? "+" : line.kind === "remove" ? "−" : "";

  const text = create("span", "diff__text", []);
  if (line.kind !== "same") {
    text.append(
      create("span", "visually-hidden", [
        line.kind === "add" ? "新增行：" : "删除行：",
      ]),
    );
  }
  text.append(line.text);

  row.append(
    create("span", "diff__num", [number === null ? "" : String(number)]),
    create("span", "diff__sign", [sign]),
    text,
  );
  return row;
}

function summary(result: DiffResult): HTMLElement {
  return create("span", "diff__summary", [
    create("span", "diff__summary-add", [`+${result.added}`]),
    create("span", "diff__summary-remove", [`−${result.removed}`]),
  ]);
}

export function createDiffView(): DiffView {
  const head = create("div", "diff__head");
  const list = create("div", "diff__list");
  list.tabIndex = 0;
  list.setAttribute("role", "region");
  list.setAttribute("aria-label", "差异行内容");
  const note = create("p", "diff__truncated", [
    `差异较大，只渲染前 ${MAX_RENDER_LINES} 行；保存与应用仍使用完整内容。`,
  ]);
  note.hidden = true;
  const element = create("div", "diff", [head, list, note]);
  let signature = "";

  function showMessage(message: string): void {
    replaceChildren(list, [create("p", "diff__empty", [message])]);
    note.hidden = true;
  }

  function update(state: UiState): void {
    const draft = state.draft;
    if (draft === null) {
      signature = "";
      replaceChildren(head, []);
      replaceChildren(list, []);
      return;
    }
    const target = targetStateFor(state.snapshot, draft.target);
    const path = target?.path ?? targetPathFor(state, draft.target);
    const targetContent = target?.content ?? null;
    const targetError = target?.error ?? null;
    const key = [
      draft.target,
      path,
      targetError ?? "",
      targetContent === null ? "\u0000missing" : targetContent,
      draft.content,
    ].join("\u0001");
    if (key === signature) {
      return;
    }
    signature = key;

    const headPath = create("span", "diff__head-path", [
      `目标 ${targetLabel(draft.target)}：${path}`,
    ]);

    if (targetError !== null) {
      replaceChildren(head, [headPath]);
      showMessage(`目标文件读取失败：${targetError}`);
      return;
    }

    const result = diffLines(targetContent ?? "", draft.content);
    replaceChildren(head, [headPath, summary(result)]);

    if (targetContent === null) {
      head.append(
        create("span", "diff__head-path", ["目标文件尚不存在，全部内容将为新增。"]),
      );
    }

    if (result.identical) {
      showMessage("内容与目标一致，没有需要写入的差异。");
      return;
    }

    const rows = result.lines.map((line) => diffRow(line));
    replaceChildren(list, rows);
    note.hidden = !result.truncated;
  }

  return { element, update };
}
