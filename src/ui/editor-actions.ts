import { checkJsonObject } from "../drafts";
import type { UiState } from "../state/store";
import { isDirty } from "../state/selectors";
import type { AppHandlers } from "./handlers";
import { button, iconButton } from "./controls";
import { create } from "./dom";

export interface EditorActionsView {
  element: HTMLElement;
  update(state: UiState): void;
}

type ActionHandlers = Pick<
  AppHandlers,
  | "onSave"
  | "onApply"
  | "onDuplicate"
  | "onImportFiles"
  | "onExport"
  | "onDelete"
  | "onRevert"
>;

export function createEditorActions(
  handlers: ActionHandlers,
): EditorActionsView {
  const save = button("保存", "filled", "save");
  const apply = button("应用", "tonal", "publish");
  save.addEventListener("click", handlers.onSave);
  apply.addEventListener("click", handlers.onApply);

  const duplicate = iconButton("content_copy", "复制预设");
  duplicate.addEventListener("click", handlers.onDuplicate);
  const importButton = iconButton("upload_file", "导入预设");
  const exportButton = iconButton("download", "导出预设");
  exportButton.addEventListener("click", handlers.onExport);
  const remove = iconButton("delete", "删除预设", true);
  remove.addEventListener("click", handlers.onDelete);
  const revert = iconButton("undo", "撤销未保存的修改");
  revert.addEventListener("click", handlers.onRevert);

  const fileInput = create("input", "visually-hidden");
  fileInput.type = "file";
  fileInput.accept = ".json,.jsonc,application/json";
  fileInput.multiple = true;
  fileInput.tabIndex = -1;
  fileInput.setAttribute("aria-label", "导入预设文件");
  importButton.addEventListener("click", () => {
    fileInput.click();
  });
  fileInput.addEventListener("change", () => {
    const files = Array.from(fileInput.files ?? []);
    if (files.length > 0) {
      handlers.onImportFiles(files);
    }
    fileInput.value = "";
  });

  const secondary = create("div", "editor__actions-secondary", [
    duplicate,
    importButton,
    exportButton,
    remove,
    revert,
  ]);

  const element = create("div", "editor__actions", [
    save,
    apply,
    secondary,
    fileInput,
  ]);

  function update(state: UiState): void {
    const draft = state.draft;
    const ready = state.status === "ready" && state.busy === null;
    const dirty = isDirty(state);
    const hasSaved = draft !== null && draft.id.length > 0;
    const jsonOk = draft !== null && checkJsonObject(draft.content).ok;

    save.disabled = !ready || draft === null;
    save.title =
      draft === null ? "没有可保存的预设" : "保存预设（Ctrl+S）";

    apply.disabled = !ready || !hasSaved || dirty;
    apply.title = dirty
      ? "请先保存修改，再应用到目标文件"
      : hasSaved
        ? "将已保存的内容写入目标文件"
        : "尚未保存的预设无法应用";

    duplicate.disabled = !ready || draft === null || !jsonOk;
    duplicate.title = !jsonOk
      ? "JSON 无效，无法复制"
      : "以当前编辑内容创建副本";

    exportButton.disabled = !ready || !hasSaved || dirty;
    exportButton.title = dirty
      ? "请先保存修改，再导出"
      : hasSaved
        ? "导出为项目内 JSON 文件"
        : "尚未保存的预设无法导出";

    remove.disabled = !ready || !hasSaved;
    remove.title = hasSaved ? "删除此预设" : "尚未保存的预设无法删除";

    revert.disabled = !ready || !dirty;
    revert.title = dirty ? "放弃未保存的修改" : "没有未保存的修改";

    importButton.disabled = !ready;
    importButton.title = ready
      ? "从 JSON 或 JSONC 文件导入预设"
      : "暂时无法导入";
  }

  return { element, update };
}
