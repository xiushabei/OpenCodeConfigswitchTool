import { getCurrentWebview } from "@tauri-apps/api/webview";
import { icon } from "../icons/icons";
import { isDesktopRuntime } from "../tauri";
import { create } from "./dom";

export interface DropZoneOptions {
  onFiles(files: readonly File[]): void;
  onPaths(paths: readonly string[]): void;
  onError(error: unknown): void;
}

export interface DropZoneView {
  element: HTMLElement;
  mount(): void;
}

function hasFiles(event: DragEvent): boolean {
  const types = event.dataTransfer?.types;
  return types !== undefined && Array.from(types).includes("Files");
}

export function createDropZone(options: DropZoneOptions): DropZoneView {
  const panel = create("div", "drop-zone__panel", [
    create("span", "drop-zone__icon", [icon("upload_file", 32)]),
    create("p", "drop-zone__title", ["松开以导入配置文件"]),
    create("p", "drop-zone__hint", [
      "支持 .json / .jsonc，可一次导入多个文件",
    ]),
  ]);
  const element = create("div", "drop-zone", [panel]);
  element.hidden = true;
  element.setAttribute("aria-live", "polite");

  function show(): void {
    element.hidden = false;
  }

  function hide(): void {
    element.hidden = true;
  }

  function mountNative(): void {
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        const payload = event.payload;
        if (payload.type === "enter" || payload.type === "over") {
          show();
          return;
        }
        hide();
        if (payload.type === "drop" && payload.paths.length > 0) {
          options.onPaths(payload.paths);
        }
      })
      .catch(options.onError);
  }

  function mountBrowser(): void {
    let depth = 0;

    function reset(): void {
      depth = 0;
      hide();
    }

    window.addEventListener("dragenter", (event) => {
      if (!hasFiles(event)) {
        return;
      }
      event.preventDefault();
      depth += 1;
      show();
    });
    window.addEventListener("dragover", (event) => {
      if (!hasFiles(event)) {
        return;
      }
      event.preventDefault();
      if (event.dataTransfer !== null) {
        event.dataTransfer.dropEffect = "copy";
      }
      show();
    });
    window.addEventListener("dragleave", () => {
      if (depth === 0) {
        return;
      }
      depth -= 1;
      if (depth === 0) {
        hide();
      }
    });
    window.addEventListener("drop", (event) => {
      if (!hasFiles(event)) {
        return;
      }
      event.preventDefault();
      reset();
      const files = Array.from(event.dataTransfer?.files ?? []);
      if (files.length > 0) {
        options.onFiles(files);
      }
    });
    window.addEventListener("dragend", reset);
    window.addEventListener("blur", reset);
  }

  function mount(): void {
    if (isDesktopRuntime()) {
      mountNative();
      return;
    }
    mountBrowser();
  }

  return { element, mount };
}
