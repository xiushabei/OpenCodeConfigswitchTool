import {
  getSoundEnabled,
  playClick,
  setSoundEnabled,
} from "../sounds";
import { create } from "./dom";

export interface SoundFieldView {
  element: HTMLElement;
  sync(): void;
  setDisabled(disabled: boolean): void;
}

export function createSoundField(): SoundFieldView {
  const input = create("input", "switch");
  input.type = "checkbox";
  input.id = "settings-sound";
  input.dataset["sound"] = "none";

  const label = create("label", "switch__row", [
    input,
    create("span", "switch__text", ["操作音效"]),
  ]);
  label.htmlFor = "settings-sound";

  const hint = create("span", "field__support", [
    "点击按钮、切换分类或修改选项时播放轻微提示音；关闭后立即静音。",
  ]);
  hint.id = "settings-sound-hint";
  input.setAttribute("aria-describedby", hint.id);

  const element = create("div", "field", [label, hint]);

  input.addEventListener("change", () => {
    setSoundEnabled(input.checked);
    if (input.checked) {
      playClick();
    }
  });

  return {
    element,
    sync: () => {
      input.checked = getSoundEnabled();
    },
    setDisabled: (disabled: boolean) => {
      input.disabled = disabled;
    },
  };
}
