import type { Draft, Settings } from "../types";
import type { EditorTab, Filter } from "../state/store";

export interface AppHandlers {
  onSelectPreset(id: string): void;
  onNewPreset(): void;
  onRefresh(): void;
  onOpenSettings(): void;
  onFilterChange(filter: Filter): void;
  onDraftChange(patch: Partial<Draft>): void;
  onTabChange(tab: EditorTab): void;
  onSave(): void;
  onApply(): void;
  onDuplicate(): void;
  onImportFiles(files: readonly File[]): void;
  onImportPaths(paths: readonly string[]): void;
  onImportCurrent(): void;
  onExport(): void;
  onRestoreStartup(): void;
  onDelete(): void;
  onRevert(): void;
  onDismissBanner(): void;
  onSaveSettings(settings: Settings): Promise<string | null>;
}
