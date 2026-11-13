import { icon, type IconName } from "../icons/icons";
import type { BusyAction, UiState } from "../state/store";
import { isDirty } from "../state/selectors";
import { create, replaceChildren } from "./dom";

const BUSY_LABELS: Record<BusyAction, string> = {
  refresh: "正在刷新目标内容…",
  save: "正在保存预设…",
  apply: "正在备份并写入目标文件…",
  delete: "正在删除预设…",
  duplicate: "正在创建副本…",
  export: "正在导出预设…",
  import: "正在导入预设…",
  restore: "正在还原至原配置…",
  settings: "正在保存设置…",
};

interface StatusContent {
  iconName: IconName;
  text: string;
  tone: "neutral" | "success" | "warning" | "error";
}

function contentFor(state: UiState): StatusContent {
  if (state.busy !== null) {
    return { iconName: "refresh", text: BUSY_LABELS[state.busy], tone: "neutral" };
  }
  switch (state.status) {
    case "loading":
      return { iconName: "info", text: "正在读取配置…", tone: "neutral" };
    case "desktop-required":
      return {
        iconName: "desktop_windows",
        text: "未连接桌面运行时，浏览器预览不会写入任何文件",
        tone: "warning",
      };
    case "error":
      return { iconName: "error", text: "读取失败", tone: "error" };
    case "ready": {
      const presets = state.snapshot?.presets.length ?? 0;
      const targets = state.snapshot?.targets.length ?? 0;
      if (isDirty(state)) {
        return { iconName: "warning", text: "有未保存的修改", tone: "warning" };
      }
      const broken =
        state.snapshot?.targets.filter((entry) => entry.error !== null)
          .length ?? 0;
      if (broken > 0) {
        return {
          iconName: "warning",
          text: `${broken} 个目标读取失败，请检查设置中的路径或刷新`,
          tone: "warning",
        };
      }
      return {
        iconName: "check_circle",
        text: `就绪 · ${presets} 个预设 · ${targets} 个目标`,
        tone: "success",
      };
    }
    default:
      throw new Error(`未处理的状态：${String(state.status)}`);
  }
}

export interface StatusBarView {
  element: HTMLElement;
  update(state: UiState): void;
}

export function createStatusBar(): StatusBarView {
  const statusIcon = create("span", "status-bar__icon", []);
  const statusText = create("span", "", ["正在启动…"]);
  const main = create("span", "status-bar__main", [statusIcon, statusText]);
  main.setAttribute("aria-live", "polite");
  const hint = create("span", "status-bar__hint", ["Ctrl+S 保存"]);
  const element = create("footer", "status-bar", [main, hint]);
  let signature = "";

  function update(state: UiState): void {
    const content = contentFor(state);
    const hintText = state.status === "ready" ? "Ctrl+S 保存" : "";
    const next = `${content.iconName}\u0000${content.text}\u0000${content.tone}\u0000${hintText}`;
    if (next === signature) {
      return;
    }
    signature = next;
    replaceChildren(statusIcon, [icon(content.iconName, 16)]);
    statusText.textContent = content.text;
    main.setAttribute("data-tone", content.tone);
    hint.textContent = hintText;
  }

  return { element, update };
}
