import { icon } from "../icons/icons";
import type { UiState } from "../state/store";
import { button, iconButton } from "./controls";
import { create } from "./dom";

export interface TopBarView {
  element: HTMLElement;
  update(state: UiState): void;
}

export interface TopBarOptions {
  onRefresh(): void;
  onOpenSettings(): void;
  onOpenModels(): void;
  onOpenChats(): void;
  onOpenFiles(files: readonly File[]): void;
  onImportCurrent(): void;
  onRestoreStartup(): void;
}

export function createTopBar(options: TopBarOptions): TopBarView {
  const subtitle = create("span", "top-bar__subtitle", ["正在读取…"]);
  const mark = create("span", "top-bar__mark", [icon("sync_alt", 22)]);
  const titles = create("div", "top-bar__titles", [
    create("h1", "top-bar__title", ["预设切换器"]),
    subtitle,
  ]);
  const brand = create("div", "top-bar__brand", [mark, titles]);

  const rootPath = create("span", "top-bar__root-path mono", ["—"]);
  const root = create("div", "top-bar__root", [
    icon("folder_open", 18),
    rootPath,
  ]);
  root.title = "项目根目录";

  const fileInput = create("input", "visually-hidden");
  fileInput.type = "file";
  fileInput.accept = ".json,.jsonc,application/json";
  fileInput.multiple = true;
  fileInput.tabIndex = -1;
  fileInput.setAttribute("aria-label", "导入配置文件");
  fileInput.addEventListener("change", () => {
    const files = Array.from(fileInput.files ?? []);
    if (files.length > 0) {
      options.onOpenFiles(files);
    }
    fileInput.value = "";
  });

  const models = button("模型列表", "outlined", "sync_alt");
  models.classList.add("top-bar__wide-action", "top-bar__models-action");
  models.setAttribute("aria-label", "模型列表");
  models.title = "查看 OpenCode 可用的真实模型列表";
  models.addEventListener("click", options.onOpenModels);

  const chats = button("聊天记录", "outlined", "description");
  chats.classList.add("top-bar__wide-action");
  chats.setAttribute("aria-label", "聊天记录");
  chats.addEventListener("click", options.onOpenChats);

  const importConfig = button("导入配置文件", "outlined", "folder_open");
  importConfig.classList.add("top-bar__wide-action");
  importConfig.setAttribute("aria-label", "导入配置文件");
  importConfig.addEventListener("click", () => {
    fileInput.click();
  });
  const importCurrent = button("导入当前配置", "outlined", "download");
  importCurrent.classList.add("top-bar__wide-action");
  importCurrent.setAttribute("aria-label", "导入当前配置");
  importCurrent.addEventListener("click", options.onImportCurrent);
  const restore = button("还原至原配置", "outlined", "undo");
  restore.classList.add("top-bar__wide-action");
  restore.setAttribute("aria-label", "还原至原配置");
  restore.addEventListener("click", options.onRestoreStartup);

  const refresh = iconButton("refresh", "刷新目标内容");
  refresh.addEventListener("click", options.onRefresh);
  const settings = iconButton("settings", "设置");
  settings.addEventListener("click", options.onOpenSettings);
  const actions = create("div", "top-bar__actions", [
    models,
    chats,
    importConfig,
    importCurrent,
    restore,
    fileInput,
    refresh,
    settings,
  ]);

  const element = create("header", "top-bar", [brand, root, actions]);
  let signature = "";

  function update(state: UiState): void {
    const rootText = state.snapshot?.root ?? "—";
    const subtitleText =
      state.snapshot !== null
        ? ""
        : state.status === "desktop-required"
          ? "当前为浏览器预览"
          : state.status === "error"
            ? "读取失败"
            : "正在读取…";
    const unavailable = state.status !== "ready" || state.busy !== null;
    const canRestore = state.snapshot?.canRestoreStartup === true;
    const restoreDisabled = unavailable || !canRestore;
    const next = [
      rootText,
      subtitleText,
      String(unavailable),
      String(restoreDisabled),
      state.busy ?? "",
      String(state.snapshot === null),
    ].join("\u0000");
    if (next === signature) {
      return;
    }
    signature = next;
    rootPath.textContent = rootText;
    root.title = `项目根目录：${rootText}`;
    root.hidden = state.snapshot === null;
    subtitle.textContent = subtitleText;
    subtitle.hidden = state.snapshot !== null;
    importConfig.disabled = unavailable;
    importConfig.title = unavailable
      ? "桌面运行时不可用或正在处理"
      : "导入 .json 或 .jsonc 配置文件";
    fileInput.disabled = unavailable;
    importCurrent.disabled = unavailable;
    importCurrent.title = unavailable
      ? "暂时无法导入当前配置"
      : "读取两个目标文件的当前内容并新增预设（每次都会追加）";
    restore.disabled = restoreDisabled;
    restore.title = canRestore
      ? "把两个目标文件还原为本次启动时的原始内容"
      : "没有可还原的原配置";
    refresh.disabled = unavailable;
    settings.disabled = unavailable;
    refresh.setAttribute(
      "title",
      unavailable ? "暂时无法刷新" : "刷新目标内容",
    );
  }

  return { element, update };
}
