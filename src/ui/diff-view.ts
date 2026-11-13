import {
  diffConfigs,
  describeConfigError,
  diffLines,
  MAX_RENDER_LINES,
} from "../diff";
import type { DiffLine, DiffResult } from "../diff";
import { targetLabel } from "../format";
import type { UiState } from "../state/store";
import { targetPathFor, targetStateFor } from "../state/selectors";
import { create, replaceChildren } from "./dom";

export interface DiffView {
  element: HTMLElement;
  update(state: UiState): void;
}

const SEMANTIC_TITLE =
  "按 JSON 语义比较：忽略注释、格式和键顺序；数组顺序参与比较；大整数按原文精确比较。";

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

function modeChip(label: string, title: string): HTMLElement {
  const chip = create("span", "diff__mode", [label]);
  chip.title = title;
  return chip;
}

export function createDiffView(): DiffView {
  const head = create("div", "diff__head");
  const list = create("div", "diff__list");
  list.tabIndex = 0;
  list.setAttribute("role", "region");
  list.setAttribute("aria-label", "差异行内容");
  const notice = create("p", "diff__note");
  notice.hidden = true;
  const note = create("p", "diff__truncated", [
    `差异较大，只渲染前 ${MAX_RENDER_LINES} 行；保存与应用仍使用完整内容。`,
  ]);
  note.hidden = true;
  const element = create("div", "diff", [head, notice, list, note]);
  let signature = "";

  function showMessage(message: string): void {
    replaceChildren(list, [create("p", "diff__empty", [message])]);
    note.hidden = true;
  }

  function setNotice(lines: readonly string[], tone: "error" | "info"): void {
    if (lines.length === 0) {
      notice.hidden = true;
      replaceChildren(notice, []);
      return;
    }
    notice.setAttribute("data-tone", tone);
    replaceChildren(notice, [lines.join("\n")]);
    notice.hidden = false;
  }

  function setHead(path: string, meta: readonly Node[]): void {
    replaceChildren(head, [
      create("span", "diff__head-path", [`目标 ${path}`]),
      create("div", "diff__head-meta", meta),
    ]);
  }

  function renderRows(result: DiffResult): void {
    const rows = result.lines.map((line) => diffRow(line));
    replaceChildren(list, rows);
    note.hidden = !result.truncated;
  }

  function update(state: UiState): void {
    const draft = state.draft;
    if (draft === null) {
      signature = "";
      element.removeAttribute("data-mode");
      setNotice([], "info");
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
      element.setAttribute("data-mode", "raw");
      setNotice([], "info");
      replaceChildren(head, [headPath]);
      showMessage(`目标文件读取失败：${targetError}`);
      return;
    }

    if (targetContent === null) {
      element.setAttribute("data-mode", "raw");
      setNotice([], "info");
      const result = diffLines("", draft.content);
      replaceChildren(head, [headPath, summary(result)]);
      head.append(
        create("span", "diff__head-path", [
          "目标文件尚不存在，全部内容将为新增。",
        ]),
      );
      if (draft.content.length === 0) {
        showMessage("目标文件尚不存在，预设内容为空。");
        return;
      }
      renderRows(result);
      return;
    }

    const diff = diffConfigs(targetContent, draft.content);
    element.setAttribute("data-mode", diff.mode);
    setHead(`${targetLabel(draft.target)}：${path}`, [
      ...(diff.mode === "semantic"
        ? [
            modeChip("JSON 语义", SEMANTIC_TITLE),
          ]
        : [
            modeChip(
              "原文逐行",
              "JSON 无效，已退回按原文逐行比较；修复语法后才能判断内容是否一致。",
            ),
          ]),
      summary(diff.result),
    ]);

    if (diff.mode === "raw") {
      const errors: string[] = [];
      if (diff.targetError !== null) {
        errors.push(describeConfigError("target", diff.targetError));
      }
      if (diff.draftError !== null) {
        errors.push(describeConfigError("draft", diff.draftError));
      }
      errors.push(
        diff.rawIdentical
          ? "两侧原文相同，但 JSON 无效，不能视为内容一致。"
          : "已退回按原文逐行比较；修复 JSON 语法后才能判断字段是否一致。",
      );
      setNotice(errors, "error");
    } else if (diff.semanticEqual && !diff.rawIdentical) {
      setNotice(
        [
          "JSON 语义一致：仅注释、格式或键顺序不同。应用仍会按原文重写目标文件。",
        ],
        "info",
      );
    } else {
      setNotice([], "info");
    }

    if (diff.result.identical) {
      if (diff.mode === "semantic") {
        showMessage(
          diff.rawIdentical
            ? "内容与目标完全一致（字节相同）。"
            : "没有字段差异。",
        );
      } else {
        showMessage("两侧原文相同，但 JSON 无效。");
      }
      return;
    }

    renderRows(diff.result);
  }

  return { element, update };
}
