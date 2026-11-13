import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/layout.css";
import "./styles/editor.css";
import "./styles/controls.css";
import "./styles/indicators.css";
import "./styles/surfaces.css";
import "./styles/diff.css";
import "./styles/drop.css";
import "./styles/models.css";
import "./styles/chat.css";

import { getCurrentWindow } from "@tauri-apps/api/window";

import * as actions from "./actions";
import { errorBanner } from "./actions/shared";
import { getState, setState, subscribe } from "./state/store";
import type { UiState } from "./state/store";
import { isDirty } from "./state/selectors";
import { isDesktopRuntime } from "./tauri";
import { applyTheme, watchSystemTheme } from "./ui/theme";
import { requireMount, create } from "./ui/dom";
import { createTopBar } from "./ui/top-bar";
import { createBanner } from "./ui/banner";
import { createPresetList } from "./ui/preset-list";
import { createEditor } from "./ui/editor";
import type { EditorView } from "./ui/editor";
import { createStatusBar } from "./ui/status-bar";
import { createConfirmDialog } from "./ui/confirm-dialog";
import { createSettingsDialog } from "./ui/settings-dialog";
import { createModelsDialog } from "./ui/models-dialog";
import { createChatView } from "./chat";
import { createFlows } from "./ui/flows";
import { createDropZone } from "./ui/drop-zone";
import type { AppHandlers } from "./ui/handlers";
import {
  hasUserInteracted,
  installSoundFeedback,
  playResult,
} from "./sounds";

function render(state: UiState): void {
  topBar.update(state);
  banner.update(state);
  presetList.update(state);
  editor.update(state);
  statusBar.update(state);
  progress.hidden = state.busy === null;
}

const root = requireMount("app");

const confirmDialog = createConfirmDialog();
const settingsDialog = createSettingsDialog(actions.persistSettings);
const modelsDialog = createModelsDialog();
const chatView = createChatView();

let editorView: EditorView | null = null;
const handlers: AppHandlers = createFlows({
  confirm: confirmDialog,
  settings: settingsDialog,
  focusName: () => {
    if (editorView !== null) {
      editorView.focusName();
    }
  },
});

const topBar = createTopBar({
  onRefresh: () => {
    handlers.onRefresh();
  },
  onOpenSettings: () => {
    handlers.onOpenSettings();
  },
  onOpenModels: () => {
    modelsDialog.open();
  },
  onOpenChats: () => {
    chatView.open();
  },
  onOpenFiles: handlers.onImportFiles,
  onImportCurrent: handlers.onImportCurrent,
  onRestoreStartup: handlers.onRestoreStartup,
});
const banner = createBanner(handlers.onDismissBanner);
const presetList = createPresetList({
  onFilterChange: handlers.onFilterChange,
  onSelect: handlers.onSelectPreset,
  onNewPreset: handlers.onNewPreset,
});
const editor = createEditor(handlers);
editorView = editor;
const statusBar = createStatusBar();

const progress = create("div", "progress", [create("div", "progress__bar")]);
progress.hidden = true;
progress.setAttribute("aria-hidden", "true");

const dropZone = createDropZone({
  onFiles: handlers.onImportFiles,
  onPaths: handlers.onImportPaths,
  onError: (error: unknown) => {
    setState({ banner: errorBanner(error, "拖放导入初始化失败") });
  },
});

const workspace = create("div", "workspace", [
  presetList.element,
  editor.element,
]);
const app = create("div", "app", [
  topBar.element,
  banner.element,
  workspace,
  statusBar.element,
  progress,
  dropZone.element,
]);
root.append(app);
document.body.append(
  confirmDialog.element,
  settingsDialog.element,
  modelsDialog.element,
  chatView.element,
);
dropZone.mount();

applyTheme("system");
watchSystemTheme(() => {
  applyTheme(getState().snapshot?.settings.theme ?? "system");
});

subscribe(render);
render(getState());

installSoundFeedback();
let bannerSignature: string | null = null;
subscribe((state) => {
  const banner = state.banner;
  const next =
    banner === null
      ? null
      : `${banner.tone}\u0000${banner.message}\u0000${banner.detail ?? ""}`;
  if (next === bannerSignature) {
    return;
  }
  bannerSignature = next;
  if (banner === null || banner.tone === "info" || !hasUserInteracted()) {
    return;
  }
  playResult(banner.tone);
});

void actions.loadSnapshot();

function destroyWindow(): void {
  getCurrentWindow()
    .destroy()
    .catch((error: unknown) => {
      setState({ banner: errorBanner(error, "关闭窗口失败") });
    });
}

function confirmClose(): Promise<boolean> {
  return confirmDialog.ask({
    title: "放弃未保存的修改？",
    message: "当前预设的修改尚未保存，关闭窗口会丢失这些修改。",
    detail: null,
    confirmLabel: "放弃并关闭",
    danger: true,
  });
}

if (isDesktopRuntime()) {
  void getCurrentWindow()
    .onCloseRequested((event) => {
      event.preventDefault();
      const state = getState();
      if (state.busy !== null) {
        return;
      }
      if (!isDirty(state)) {
        destroyWindow();
        return;
      }
      void confirmClose().then((confirmed) => {
        if (confirmed) {
          destroyWindow();
        }
      });
    })
    .catch((error: unknown) => {
      setState({ banner: errorBanner(error, "关闭窗口保护初始化失败") });
    });
}

window.addEventListener("keydown", (event) => {
  if (!(event.ctrlKey || event.metaKey) || event.key.toLowerCase() !== "s") {
    return;
  }
  if (document.querySelector("dialog[open]") !== null) {
    return;
  }
  const state = getState();
  if (state.status !== "ready" || state.busy !== null) {
    return;
  }
  event.preventDefault();
  handlers.onSave();
});
