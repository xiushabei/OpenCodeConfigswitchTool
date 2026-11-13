import type { ResolvedTheme, Theme } from "../types";

const DARK_QUERY = "(prefers-color-scheme: dark)";

function darkQuery(): MediaQueryList | null {
  if (typeof window.matchMedia !== "function") {
    return null;
  }
  return window.matchMedia(DARK_QUERY);
}

export function resolveTheme(preference: Theme): ResolvedTheme {
  if (preference === "system") {
    return darkQuery()?.matches === true ? "dark" : "light";
  }
  return preference;
}

export function applyTheme(preference: Theme): void {
  document.documentElement.setAttribute(
    "data-theme",
    resolveTheme(preference),
  );
}

export function watchSystemTheme(onChange: () => void): void {
  const query = darkQuery();
  if (query === null) {
    return;
  }
  query.addEventListener("change", () => {
    onChange();
  });
}
