import * as actions from "../actions";
import { targetLabel } from "../format";
import { isDirty, targetPathFor, targetStateFor } from "../state/selectors";
import { getState } from "../state/store";
import type { AppHandlers } from "./handlers";
import type { ConfirmDialogView } from "./confirm-dialog";
import type { SettingsDialogView } from "./settings-dialog";

export interface FlowDeps {
  confirm: ConfirmDialogView;
  settings: SettingsDialogView;
  focusName(): void;
}

export function createFlows(deps: FlowDeps): AppHandlers {
  async function guardDiscard(): Promise<boolean> {
    if (!isDirty(getState())) {
      return true;
    }
    return deps.confirm.ask({
      title: "放弃未保存的修改？",
      message: "当前预设的修改尚未保存，继续操作会丢失这些修改。",
      detail: null,
      confirmLabel: "放弃修改",
      danger: true,
    });
  }

  async function choosePreset(id: string): Promise<void> {
    if (getState().selectedId === id) {
      return;
    }
    if (!(await guardDiscard())) {
      return;
    }
    actions.selectPreset(id);
  }

  async function createPreset(): Promise<void> {
    if (!(await guardDiscard())) {
      return;
    }
    actions.startNewPreset();
    deps.focusName();
  }

  async function refresh(): Promise<void> {
    if (!(await guardDiscard())) {
      return;
    }
    await actions.loadSnapshot();
  }

  function openSettings(): void {
    const snapshot = getState().snapshot;
    if (snapshot === null) {
      return;
    }
    deps.settings.open(snapshot.settings);
  }

  async function apply(): Promise<void> {
    const state = getState();
    const draft = state.draft;
    if (draft === null || draft.id.length === 0) {
      return;
    }
    if (isDirty(state)) {
      await actions.applyCurrent();
      return;
    }
    const target = targetStateFor(state.snapshot, draft.target);
    const path = target?.path ?? targetPathFor(state, draft.target);
    const confirmed = await deps.confirm.ask({
      title: "应用预设？",
      message: `将把「${draft.name}」写入 ${targetLabel(draft.target)} 目标文件。写入前会备份现有内容；写入完成后需要重启 OpenCode 让配置生效。`,
      detail: path,
      confirmLabel: "应用",
      danger: false,
    });
    if (confirmed) {
      await actions.applyCurrent();
    }
  }

  async function importFiles(files: readonly File[]): Promise<void> {
    if (files.length === 0 || !(await guardDiscard())) {
      return;
    }
    await actions.importFiles(files);
  }

  async function importPaths(paths: readonly string[]): Promise<void> {
    if (paths.length === 0 || !(await guardDiscard())) {
      return;
    }
    await actions.importPaths(paths);
  }

  async function importCurrent(): Promise<void> {
    if (!(await guardDiscard())) {
      return;
    }
    await actions.importCurrentConfigs();
  }

  async function restoreStartup(): Promise<void> {
    if (!(await guardDiscard())) {
      return;
    }
    const state = getState();
    const snapshot = state.snapshot;
    if (snapshot === null || !snapshot.canRestoreStartup) {
      return;
    }
    const confirmed = await deps.confirm.ask({
      title: "还原至原配置？",
      message:
        "将把 OpenCode 和 OMO 还原到本次启动时锁定的原始路径与原始内容；之后修改设置也不会改变还原目标。还原前会备份当前文件，启动时缺失的文件会被移除。还原后需要重启 OpenCode。",
      detail: null,
      confirmLabel: "还原",
      danger: true,
    });
    if (confirmed) {
      await actions.restoreStartup();
    }
  }

  async function remove(): Promise<void> {
    const draft = getState().draft;
    if (draft === null || draft.id.length === 0) {
      return;
    }
    const confirmed = await deps.confirm.ask({
      title: "删除预设？",
      message: `将删除预设「${draft.name}」。不会修改目标配置文件；未保存的修改也会一并丢弃。`,
      detail: null,
      confirmLabel: "删除",
      danger: true,
    });
    if (confirmed) {
      await actions.deleteSelected();
    }
  }

  async function revert(): Promise<void> {
    if (!isDirty(getState())) {
      return;
    }
    const confirmed = await deps.confirm.ask({
      title: "撤销未保存的修改？",
      message: "当前修改将丢失，内容恢复到上次保存的状态。",
      detail: null,
      confirmLabel: "撤销修改",
      danger: true,
    });
    if (confirmed) {
      actions.revertDraft();
    }
  }

  function isBusy(): boolean {
    return getState().busy !== null;
  }

  return {
    onSelectPreset: (id) => {
      if (!isBusy()) {
        void choosePreset(id);
      }
    },
    onNewPreset: () => {
      if (!isBusy()) {
        void createPreset();
      }
    },
    onRefresh: () => {
      if (!isBusy()) {
        void refresh();
      }
    },
    onOpenSettings: () => {
      if (!isBusy()) {
        openSettings();
      }
    },
    onFilterChange: actions.setFilter,
    onDraftChange: actions.updateDraft,
    onTabChange: actions.setTab,
    onSave: () => {
      if (!isBusy()) {
        void actions.saveDraft();
      }
    },
    onApply: () => {
      if (!isBusy()) {
        void apply();
      }
    },
    onDuplicate: () => {
      if (!isBusy()) {
        void actions.duplicateCurrent();
      }
    },
    onImportFiles: (files) => {
      if (!isBusy()) {
        void importFiles(files);
      }
    },
    onImportPaths: (paths) => {
      if (!isBusy()) {
        void importPaths(paths);
      }
    },
    onImportCurrent: () => {
      if (!isBusy()) {
        void importCurrent();
      }
    },
    onRestoreStartup: () => {
      if (!isBusy()) {
        void restoreStartup();
      }
    },
    onExport: () => {
      if (!isBusy()) {
        void actions.exportCurrent();
      }
    },
    onDelete: () => {
      if (!isBusy()) {
        void remove();
      }
    },
    onRevert: () => {
      if (!isBusy()) {
        void revert();
      }
    },
    onDismissBanner: actions.dismissBanner,
    onSaveSettings: actions.persistSettings,
  };
}
