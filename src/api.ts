import {
  parseApplyResult,
  parseExportPath,
  parseImportContent,
  parseModelCatalogue,
  parsePatchedContent,
  parseSnapshot,
} from "./guards";
import { invokeCommand } from "./tauri";
import type { ModelCatalogue, OmoSection } from "./models/types";
import type { ApplyResult, Preset, Settings, Snapshot } from "./types";

const COMMANDS = {
  snapshot: "snapshot",
  savePreset: "save_preset",
  deletePreset: "delete_preset",
  saveSettings: "save_settings",
  applyPreset: "apply_preset",
  exportPreset: "export_preset",
  restoreStartup: "restore_startup",
  importCurrentConfigs: "import_current_configs",
  readImportFile: "read_import_file",
  normalizeImportContent: "normalize_import_content",
  listModels: "list_models",
  patchOmoModel: "patch_omo_model",
} as const;

export async function fetchSnapshot(): Promise<Snapshot> {
  return parseSnapshot(await invokeCommand(COMMANDS.snapshot));
}

export async function savePresetCommand(preset: Preset): Promise<Snapshot> {
  return parseSnapshot(await invokeCommand(COMMANDS.savePreset, { preset }));
}

export async function deletePresetCommand(id: string): Promise<Snapshot> {
  return parseSnapshot(await invokeCommand(COMMANDS.deletePreset, { id }));
}

export async function saveSettingsCommand(
  settings: Settings,
): Promise<Snapshot> {
  return parseSnapshot(await invokeCommand(COMMANDS.saveSettings, { settings }));
}

export async function applyPresetCommand(id: string): Promise<ApplyResult> {
  return parseApplyResult(
    await invokeCommand(COMMANDS.applyPreset, { id }),
  );
}

export async function exportPresetCommand(id: string): Promise<string> {
  return parseExportPath(
    await invokeCommand(COMMANDS.exportPreset, { id }),
  );
}

export async function restoreStartupCommand(): Promise<ApplyResult> {
  return parseApplyResult(await invokeCommand(COMMANDS.restoreStartup));
}

export async function importCurrentConfigsCommand(): Promise<Snapshot> {
  return parseSnapshot(await invokeCommand(COMMANDS.importCurrentConfigs));
}

export async function readImportFileCommand(path: string): Promise<string> {
  return parseImportContent(
    await invokeCommand(COMMANDS.readImportFile, { path }),
  );
}

export async function normalizeImportContentCommand(
  content: string,
): Promise<string> {
  return parseImportContent(
    await invokeCommand(COMMANDS.normalizeImportContent, { content }),
  );
}

export async function listModelsCommand(
  refresh: boolean,
): Promise<ModelCatalogue> {
  return parseModelCatalogue(
    await invokeCommand(COMMANDS.listModels, { refresh }),
  );
}

export async function patchOmoModelCommand(
  content: string,
  section: OmoSection,
  key: string,
  model: string | null,
): Promise<string> {
  return parsePatchedContent(
    await invokeCommand(COMMANDS.patchOmoModel, {
      content,
      section,
      key,
      model,
    }),
  );
}
