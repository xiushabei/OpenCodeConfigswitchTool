export type Target = "opencode" | "omo";
export type Theme = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";

export interface TargetPaths {
  opencode: string;
  omo: string;
}

export interface Settings {
  targets: TargetPaths;
  theme: Theme;
}

export interface Preset {
  id: string;
  name: string;
  description: string;
  target: Target;
  content: string;
}

export interface TargetState {
  target: Target;
  path: string;
  content: string | null;
  error: string | null;
  activePresetIds: string[];
}

export interface Snapshot {
  settings: Settings;
  presets: Preset[];
  targets: TargetState[];
  root: string;
  startupWarnings: string[];
  canRestoreStartup: boolean;
}

export interface ApplyResult {
  snapshot: Snapshot;
  backup: string | null;
  message: string;
}

export interface Draft {
  id: string;
  name: string;
  description: string;
  target: Target;
  content: string;
}

export const TARGETS: readonly Target[] = ["opencode", "omo"];

export function isTarget(value: unknown): value is Target {
  return value === "opencode" || value === "omo";
}

export function isTheme(value: unknown): value is Theme {
  return value === "system" || value === "light" || value === "dark";
}
