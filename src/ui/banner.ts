import { icon, type IconName } from "../icons/icons";
import type { BannerTone, UiState } from "../state/store";
import { iconButton } from "./controls";
import { create, replaceChildren } from "./dom";

const TONE_ICON: Record<BannerTone, IconName> = {
  success: "check_circle",
  error: "error",
  info: "info",
};

export interface BannerView {
  element: HTMLElement;
  update(state: UiState): void;
}

export function createBanner(onDismiss: () => void): BannerView {
  const element = create("div", "banner-region");
  let signature = "";

  function update(state: UiState): void {
    const banner = state.banner;
    const next =
      banner === null
        ? ""
        : `${banner.tone}\u0000${banner.message}\u0000${banner.detail ?? ""}`;
    if (next === signature) {
      return;
    }
    signature = next;
    if (banner === null) {
      replaceChildren(element, []);
      return;
    }

    const close = iconButton("close", "关闭提示");
    close.classList.add("banner__close");
    close.addEventListener("click", onDismiss);

    const body = create("div", "banner__body", [
      create("p", "banner__message", [banner.message]),
    ]);
    if (banner.detail !== null) {
      body.append(create("p", "banner__detail", [banner.detail]));
    }

    const node = create("div", "banner", [
      create("span", "banner__icon", [icon(TONE_ICON[banner.tone])]),
      body,
      close,
    ]);
    node.setAttribute("data-tone", banner.tone);
    node.setAttribute("role", banner.tone === "error" ? "alert" : "status");
    replaceChildren(element, [node]);
  }

  return { element, update };
}
