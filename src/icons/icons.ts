import { ICON_PATHS, type IconName } from "./paths";

const SVG_NS = "http://www.w3.org/2000/svg";

export type { IconName };

/**
 * Material Symbols Outlined, rendered from local official path data so the
 * interface never depends on a web font or network request.
 */
export function icon(name: IconName, size: number = 20): SVGSVGElement {
  const svg = document.createElementNS(SVG_NS, "svg");
  svg.setAttribute("viewBox", "0 -960 960 960");
  svg.setAttribute("width", String(size));
  svg.setAttribute("height", String(size));
  svg.setAttribute("aria-hidden", "true");
  svg.setAttribute("focusable", "false");
  svg.classList.add("icon");
  svg.style.setProperty("inline-size", `${size}px`);
  svg.style.setProperty("block-size", `${size}px`);
  const path = document.createElementNS(SVG_NS, "path");
  path.setAttribute("d", ICON_PATHS[name]);
  svg.append(path);
  return svg;
}
