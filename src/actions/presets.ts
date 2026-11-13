import {
  applyPresetCommand,
  deletePresetCommand,
  exportPresetCommand,
  savePresetCommand,
} from "../api";
import { draftFromPreset, validateDraft } from "../drafts";
import { ContractError } from "../guards";
import { getState, setState } from "../state/store";
import { isDirty } from "../state/selectors";
import type { Preset } from "../types";
import {
  errorBanner,
  filterFor,
  findSaved,
  infoBanner,
  jsonErrorFor,
  selectionFor,
} from "./shared";

export async function saveDraft(): Promise<void> {
  const state = getState();
  const draft = state.draft;
  if (draft === null) {
    return;
  }
  const validation = validateDraft(draft);
  if (!validation.ok) {
    setState({
      jsonError:
        validation.field === "content" ? validation.message : state.jsonError,
      banner: errorBanner(validation.message, "无法保存"),
    });
    return;
  }
  const created = draft.id.length === 0;
  const beforeIds = new Set((state.snapshot?.presets ?? []).map((p) => p.id));
  setState({ busy: "save", banner: null });
  try {
    const snapshot = await savePresetCommand(validation.preset);
    const saved = findSaved(snapshot, validation.preset, beforeIds);
    if (saved === null) {
      throw new ContractError("保存后未找到对应预设，请刷新确认。");
    }
    setState({
      snapshot,
      busy: null,
      selectedId: saved.id,
      draft: draftFromPreset(saved),
      baseline: draftFromPreset(saved),
      jsonError: jsonErrorFor(saved.content),
      tab: "edit",
      filter: filterFor(state, saved),
      selectionToken: getState().selectionToken + 1,
      banner: {
        tone: "success",
        message: created
          ? `已创建预设「${saved.name}」。`
          : `已保存预设「${saved.name}」。`,
        detail: null,
      },
    });
  } catch (error) {
    setState({ busy: null, banner: errorBanner(error, "保存失败") });
  }
}

export async function applyCurrent(): Promise<void> {
  const state = getState();
  const draft = state.draft;
  if (draft === null || draft.id.length === 0) {
    return;
  }
  if (isDirty(state)) {
    setState({ banner: infoBanner("请先保存修改，再应用到目标文件。") });
    return;
  }
  setState({ busy: "apply", banner: null });
  try {
    const result = await applyPresetCommand(draft.id);
    setState({
      snapshot: result.snapshot,
      busy: null,
      banner: {
        tone: "success",
        message: result.message,
        detail: result.backup === null ? null : `备份：${result.backup}`,
      },
    });
  } catch (error) {
    setState({ busy: null, banner: errorBanner(error, "应用失败") });
  }
}

export async function duplicateCurrent(): Promise<void> {
  const state = getState();
  const draft = state.draft;
  if (draft === null) {
    return;
  }
  const validation = validateDraft(draft);
  if (!validation.ok) {
    setState({ banner: errorBanner(validation.message, "无法复制") });
    return;
  }
  const hadUnsaved = isDirty(state);
  const beforeIds = new Set((state.snapshot?.presets ?? []).map((p) => p.id));
  const copy: Preset = {
    ...validation.preset,
    id: "",
    name: `${validation.preset.name} 副本`,
  };
  setState({ busy: "duplicate", banner: null });
  try {
    const snapshot = await savePresetCommand(copy);
    const saved = findSaved(snapshot, copy, beforeIds);
    if (saved === null) {
      throw new ContractError("复制后未找到新预设，请刷新确认。");
    }
    setState({
      snapshot,
      busy: null,
      ...selectionFor(snapshot, saved.id),
      tab: "edit",
      filter: filterFor(state, saved),
      selectionToken: getState().selectionToken + 1,
      banner: {
        tone: "success",
        message: `已创建副本「${saved.name}」。`,
        detail: hadUnsaved ? "副本包含当前未保存的编辑内容。" : null,
      },
    });
  } catch (error) {
    setState({ busy: null, banner: errorBanner(error, "复制失败") });
  }
}

export async function deleteSelected(): Promise<void> {
  const state = getState();
  const draft = state.draft;
  if (draft === null || draft.id.length === 0) {
    return;
  }
  const removedName = draft.name.trim().length > 0 ? draft.name : "未命名预设";
  setState({ busy: "delete", banner: null });
  try {
    const snapshot = await deletePresetCommand(draft.id);
    const next =
      (state.filter !== "all"
        ? snapshot.presets.find((preset) => preset.target === state.filter)
        : null) ??
      snapshot.presets[0] ??
      null;
    let selection = selectionFor(snapshot, next?.id ?? null);
    let filter = state.filter;
    if (selection.draft === null && snapshot.presets.length > 0) {
      selection = selectionFor(snapshot, snapshot.presets[0]?.id ?? null);
      filter = "all";
    }
    setState({
      snapshot,
      busy: null,
      ...selection,
      tab: "edit",
      filter,
      selectionToken: getState().selectionToken + 1,
      banner: {
        tone: "success",
        message: `已删除预设「${removedName}」。`,
        detail: "目标配置文件未被修改。",
      },
    });
  } catch (error) {
    setState({ busy: null, banner: errorBanner(error, "删除失败") });
  }
}

export async function exportCurrent(): Promise<void> {
  const state = getState();
  const draft = state.draft;
  if (draft === null || draft.id.length === 0) {
    return;
  }
  if (isDirty(state)) {
    setState({ banner: infoBanner("请先保存修改，再导出预设。") });
    return;
  }
  setState({ busy: "export", banner: null });
  try {
    const path = await exportPresetCommand(draft.id);
    setState({
      busy: null,
      banner: {
        tone: "success",
        message: `已导出预设「${draft.name}」。`,
        detail: path,
      },
    });
  } catch (error) {
    setState({ busy: null, banner: errorBanner(error, "导出失败") });
  }
}
