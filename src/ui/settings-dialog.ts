import { isTheme, type Settings } from "../types";
import { icon } from "../icons/icons";
import { button, iconButton } from "./controls";
import { create } from "./dom";
import { createSoundField } from "./sound-field";

export interface SettingsDialogView {
  element: HTMLDialogElement;
  open(settings: Settings): void;
}

export function createSettingsDialog(
  onSave: (settings: Settings) => Promise<string | null>,
): SettingsDialogView {
  const dialog = create("dialog", "dialog");
  dialog.id = "settings-dialog";

  const title = create("h2", "dialog__title", ["设置"]);
  title.id = "settings-title";
  const close = iconButton("close", "关闭设置");
  const head = create("div", "dialog__head", [title, close]);

  const opencodeInput = create("input", "field__control");
  opencodeInput.type = "text";
  opencodeInput.id = "settings-opencode";
  opencodeInput.autocomplete = "off";
  opencodeInput.spellcheck = false;
  const opencodeLabel = create("label", "field__label", ["OpenCode 目标路径"]);
  opencodeLabel.htmlFor = "settings-opencode";
  const opencodeField = create("div", "field", [
    opencodeLabel,
    opencodeInput,
    create("span", "field__support", [
      "支持 ~/… 或项目内相对路径；绝对路径用于明确指定的位置。",
    ]),
  ]);

  const omoInput = create("input", "field__control");
  omoInput.type = "text";
  omoInput.id = "settings-omo";
  omoInput.autocomplete = "off";
  omoInput.spellcheck = false;
  const omoLabel = create("label", "field__label", ["OMO 目标路径"]);
  omoLabel.htmlFor = "settings-omo";
  const omoField = create("div", "field", [
    omoLabel,
    omoInput,
    create("span", "field__support", [
      "保存时会校验空值、无效路径以及与应用文件的重名冲突。",
    ]),
  ]);

  const soundField = createSoundField();

  const themeFieldset = create("fieldset", "field field--plain");
  const legend = create("legend", "field__label", ["主题"]);
  const themeChoice = create("div", "theme-choice");
  const themeInputs = new Map<string, HTMLInputElement>();
  const themeOptions: readonly {
    value: string;
    label: string;
    icon: "brightness_auto" | "light_mode" | "dark_mode";
  }[] = [
    { value: "system", label: "跟随系统", icon: "brightness_auto" },
    { value: "light", label: "浅色", icon: "light_mode" },
    { value: "dark", label: "深色", icon: "dark_mode" },
  ];
  for (const option of themeOptions) {
    const input = create("input", "");
    input.type = "radio";
    input.name = "theme";
    input.value = option.value;
    themeInputs.set(option.value, input);
    const label = create("label", "theme-choice__option", [
      input,
      icon(option.icon, 18),
      create("span", "", [option.label]),
    ]);
    themeChoice.append(label);
  }
  themeFieldset.append(legend, themeChoice);

  const errorText = create("span", "", [""]);
  const errorLine = create("p", "dialog__error", [errorText]);
  errorLine.hidden = true;
  errorLine.setAttribute("role", "alert");

  const cancel = button("取消", "text");
  const save = button("保存设置", "filled", "save");
  save.type = "submit";
  const actions = create("div", "dialog__actions", [cancel, save]);

  const form = create("form", "dialog__form");
  form.setAttribute("novalidate", "");
  form.append(
    head,
    create("div", "dialog__body", [
      opencodeField,
      omoField,
      themeFieldset,
      soundField.element,
      errorLine,
    ]),
    actions,
  );
  dialog.append(form);
  dialog.setAttribute("aria-labelledby", "settings-title");

  let opener: HTMLElement | null = null;
  let saving = false;

  function selectedTheme(): Settings["theme"] | null {
    for (const [value, input] of themeInputs) {
      if (input.checked && isTheme(value)) {
        return value;
      }
    }
    return null;
  }

  function showError(message: string): void {
    errorText.textContent = message;
    errorLine.hidden = false;
  }

  function clearError(): void {
    errorText.textContent = "";
    errorLine.hidden = true;
  }

  function setSaving(next: boolean): void {
    saving = next;
    opencodeInput.readOnly = next;
    omoInput.readOnly = next;
    for (const input of themeInputs.values()) {
      input.disabled = next;
    }
    soundField.setDisabled(next);
    save.disabled = next;
    cancel.disabled = next;
    close.disabled = next;
    form.setAttribute("aria-busy", String(next));
  }

  function closeDialog(): void {
    if (dialog.open) {
      dialog.close("cancel");
    }
  }

  close.addEventListener("click", closeDialog);
  cancel.addEventListener("click", closeDialog);

  dialog.addEventListener("cancel", (event) => {
    if (saving) {
      event.preventDefault();
    }
  });

  dialog.addEventListener("close", () => {
    if (opener !== null) {
      opener.focus();
    }
    opener = null;
  });

  form.addEventListener("submit", (event) => {
    event.preventDefault();
    if (saving) {
      return;
    }
    const opencode = opencodeInput.value.trim();
    const omo = omoInput.value.trim();
    const theme = selectedTheme();
    if (opencode.length === 0 || omo.length === 0) {
      showError("两个目标路径都不能为空。");
      if (opencode.length === 0) {
        opencodeInput.focus();
      } else {
        omoInput.focus();
      }
      return;
    }
    if (theme === null) {
      showError("请选择主题。");
      return;
    }
    clearError();
    setSaving(true);
    void onSave({ targets: { opencode, omo }, theme }).then((error) => {
      if (error !== null) {
        setSaving(false);
        showError(error);
        return;
      }
      setSaving(false);
      dialog.close("saved");
    });
  });

  function open(settings: Settings): void {
    clearError();
    setSaving(false);
    opencodeInput.value = settings.targets.opencode;
    omoInput.value = settings.targets.omo;
    for (const [value, input] of themeInputs) {
      input.checked = value === settings.theme;
    }
    soundField.sync();
    opener =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    dialog.showModal();
    opencodeInput.focus();
    opencodeInput.select();
  }

  return { element: dialog, open };
}
