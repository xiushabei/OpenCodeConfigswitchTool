import { invoke } from "@tauri-apps/api/core";

/** Raised in a plain browser preview where Tauri IPC does not exist. */
export class DesktopUnavailableError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "DesktopUnavailableError";
  }
}

/** Raised when a Tauri command rejects. */
export class CommandError extends Error {
  readonly command: string;

  constructor(command: string, message: string) {
    super(message);
    this.name = "CommandError";
    this.command = command;
  }
}

export function isDesktopRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function errorMessage(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === "object" && error !== null && "message" in error) {
    const message = error.message;
    if (typeof message === "string") {
      return message;
    }
  }
  return "未知错误";
}

export async function invokeCommand(
  command: string,
  args?: Record<string, unknown>,
): Promise<unknown> {
  if (!isDesktopRuntime()) {
    throw new DesktopUnavailableError(
      "未检测到桌面运行时。请启动 Tauri 桌面应用后再使用此界面。",
    );
  }
  try {
    return await invoke<unknown>(command, args);
  } catch (error) {
    throw new CommandError(command, errorMessage(error));
  }
}
