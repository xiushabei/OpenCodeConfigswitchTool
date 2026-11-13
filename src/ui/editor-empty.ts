import { icon, type IconName } from "../icons/icons";
import type { UiState } from "../state/store";
import type { AppHandlers } from "./handlers";
import { button } from "./controls";
import { create, replaceChildren } from "./dom";

export interface EditorEmptyView {
  element: HTMLElement;
  update(state: UiState): void;
}

type EmptyHandlers = Pick<AppHandlers, "onRefresh" | "onNewPreset">;

interface EmptyContent {
  iconName: IconName;
  title: string;
  text: string;
  action: "retry" | "new" | "none";
}

function contentFor(state: UiState): EmptyContent {
  switch (state.status) {
    case "loading":
      return {
        iconName: "description",
        title: "正在读取配置",
        text: "正在读取项目设置、预设和目标文件。",
        action: "none",
      };
    case "desktop-required":
      return {
        iconName: "desktop_windows",
        title: "需要桌面应用",
        text: "此界面通过 Tauri 读写本地配置。浏览器预览不会伪造预设或写入结果，请启动桌面应用后重试。",
        action: "retry",
      };
    case "error":
      return {
        iconName: "error",
        title: "无法读取预设",
        text: state.error ?? "未知的读取错误。",
        action: "retry",
      };
    case "ready": {
      const count = state.snapshot?.presets.length ?? 0;
      if (count === 0) {
        return {
          iconName: "description",
          title: "还没有预设",
          text: "新建一个预设，编辑 JSON 后保存，再应用到 OpenCode 或 OMO 的目标文件。",
          action: "new",
        };
      }
      return {
        iconName: "description",
        title: "未选择预设",
        text: "从左侧选择一个预设开始编辑，或新建一个。",
        action: "new",
      };
    }
    default:
      throw new Error(`未处理的状态：${String(state.status)}`);
  }
}

export function createEditorEmpty(handlers: EmptyHandlers): EditorEmptyView {
  const element = create("div", "editor__empty");
  let signature = "";

  function update(state: UiState): void {
    const content = contentFor(state);
    const next = [
      content.iconName,
      content.title,
      content.text,
      content.action,
      state.busy ?? "",
      String(state.snapshot?.presets.length ?? 0),
    ].join("\u0000");
    if (next === signature) {
      return;
    }
    signature = next;

    const actions = create("div", "empty-state__actions");
    if (content.action === "retry") {
      const retry = button(
        "重试",
        state.status === "desktop-required" ? "outlined" : "tonal",
        "refresh",
      );
      retry.addEventListener("click", handlers.onRefresh);
      retry.disabled = state.busy !== null;
      actions.append(retry);
    } else if (content.action === "new") {
      const createButton = button("新建预设", "tonal", "add");
      createButton.addEventListener("click", handlers.onNewPreset);
      createButton.disabled = state.busy !== null;
      actions.append(createButton);
    }

    const children: HTMLElement[] = [
      create("span", "empty-state__icon", [icon(content.iconName, 26)]),
      create("h2", "empty-state__title", [content.title]),
      create("p", "empty-state__text", [content.text]),
    ];
    if (actions.childElementCount > 0) {
      children.push(actions);
    }
    replaceChildren(element, [create("div", "empty-state", children)]);
  }

  return { element, update };
}
