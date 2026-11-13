import {
  normalizeImportContentCommand,
  readImportFileCommand,
  savePresetCommand,
} from "../api";
import { ContractError } from "../guards";
import {
  isImportableFileName,
  parseImportFile,
  targetFromFileName,
} from "../import-file";
import { getState, setState } from "../state/store";
import type { UiState } from "../state/store";
import type { Preset, Snapshot, Target } from "../types";
import { errorMessage } from "../tauri";
import { filterFor, findSaved, selectionFor } from "./shared";

interface ImportSource {
  label: string;
  target: Target;
  load(): Promise<string>;
}

function fallbackTarget(): Target {
  const state = getState();
  return (
    state.draft?.target ??
    (state.filter === "all" ? "opencode" : state.filter)
  );
}

function invalidExtension(label: string): string {
  return `${label}：仅支持 .json 或 .jsonc 文件。`;
}

function fileSources(files: readonly File[]): {
  sources: ImportSource[];
  failures: string[];
} {
  const fallback = fallbackTarget();
  const sources: ImportSource[] = [];
  const failures: string[] = [];
  for (const file of files) {
    if (!isImportableFileName(file.name)) {
      failures.push(invalidExtension(file.name));
      continue;
    }
    sources.push({
      label: file.name,
      target: targetFromFileName(file.name, fallback),
      load: async () => {
        const text = await file.text();
        return normalizeImportContentCommand(text);
      },
    });
  }
  return { sources, failures };
}

function pathSources(paths: readonly string[]): {
  sources: ImportSource[];
  failures: string[];
} {
  const fallback = fallbackTarget();
  const sources: ImportSource[] = [];
  const failures: string[] = [];
  for (const path of paths) {
    if (!isImportableFileName(path)) {
      failures.push(invalidExtension(path));
      continue;
    }
    sources.push({
      label: path,
      target: targetFromFileName(path, fallback),
      load: async () => readImportFileCommand(path),
    });
  }
  return { sources, failures };
}

function failureBanner(failures: readonly string[]): UiState["banner"] {
  return {
    tone: "error",
    message: `导入失败：${failures.length} 个文件无法导入。`,
    detail: failures.join("\n"),
  };
}

function successMessage(names: readonly string[]): string {
  if (names.length === 1) {
    return `已导入预设「${names[0] ?? ""}」。`;
  }
  return `已导入 ${names.length} 个预设。`;
}

async function runImport(
  sources: readonly ImportSource[],
  initialFailures: readonly string[],
): Promise<void> {
  const failures = [...initialFailures];
  if (sources.length === 0) {
    setState({ banner: failureBanner(failures) });
    return;
  }

  const state = getState();
  setState({ busy: "import", banner: null });

  let latest: Snapshot | null = state.snapshot;
  let lastSaved: Preset | null = null;
  const importedNames: string[] = [];

  for (const source of sources) {
    try {
      const text = await source.load();
      const parsed = parseImportFile(text, source.label, source.target);
      if (!parsed.ok) {
        failures.push(`${source.label}：${parsed.message}`);
        continue;
      }
      const beforeIds = new Set(
        (latest?.presets ?? []).map((preset) => preset.id),
      );
      const preset: Preset = { id: "", ...parsed.payload };
      const snapshot = await savePresetCommand(preset);
      latest = snapshot;
      const saved = findSaved(snapshot, preset, beforeIds);
      if (saved === null) {
        throw new ContractError("导入后未找到新预设，请刷新确认。");
      }
      lastSaved = saved;
      importedNames.push(saved.name);
    } catch (error) {
      failures.push(`${source.label}：${errorMessage(error)}`);
    }
  }

  const patch: Partial<UiState> = { busy: null };
  if (latest !== null) {
    patch.snapshot = latest;
  }
  if (lastSaved !== null && latest !== null) {
    patch.selectionToken = getState().selectionToken + 1;
    patch.tab = "edit";
    Object.assign(patch, selectionFor(latest, lastSaved.id));
    patch.filter = filterFor(state, lastSaved);
  }

  if (failures.length === 0) {
    patch.banner = {
      tone: "success",
      message: successMessage(importedNames),
      detail: null,
    };
  } else {
    patch.banner = {
      tone: "error",
      message: `导入完成：成功 ${importedNames.length} 个，失败 ${failures.length} 个。`,
      detail: failures.join("\n"),
    };
  }
  setState(patch);
}

export async function importFiles(files: readonly File[]): Promise<void> {
  const { sources, failures } = fileSources(files);
  await runImport(sources, failures);
}

export async function importPaths(paths: readonly string[]): Promise<void> {
  const { sources, failures } = pathSources(paths);
  await runImport(sources, failures);
}
