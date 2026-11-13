import { importCurrentConfigsCommand } from "../api";
import { targetLabel } from "../format";
import { getState, setState } from "../state/store";
import type { UiState } from "../state/store";
import { errorBanner, filterFor, selectionFor } from "./shared";

export async function importCurrentConfigs(): Promise<void> {
  const state = getState();
  const beforeIds = new Set(
    (state.snapshot?.presets ?? []).map((preset) => preset.id),
  );
  setState({ busy: "import", banner: null });
  try {
    const snapshot = await importCurrentConfigsCommand();
    const added = snapshot.presets.filter(
      (preset) => !beforeIds.has(preset.id),
    );
    const last = added.at(-1) ?? null;
    const targetFailures = snapshot.targets
      .filter((target) => target.error !== null)
      .map((target) => `${targetLabel(target.target)}：${target.error ?? ""}`);
    const failures = [...new Set([
      ...targetFailures,
      ...snapshot.startupWarnings,
    ])];
    const patch: Partial<UiState> = {
      snapshot,
      busy: null,
      banner: {
        tone:
          failures.length > 0
            ? "error"
            : added.length > 0
              ? "success"
              : "info",
        message:
          failures.length > 0
            ? `导入当前配置完成：新增 ${added.length} 个，失败 ${failures.length} 个目标。`
            : added.length > 0
              ? `已导入 ${added.length} 个当前配置。`
              : "未发现可导入的当前配置。",
        detail: failures.length === 0 ? null : failures.join("\n"),
      },
    };
    if (last !== null) {
      patch.tab = "edit";
      patch.selectionToken = getState().selectionToken + 1;
      Object.assign(patch, selectionFor(snapshot, last.id));
      patch.filter = filterFor(state, last);
    }
    setState(patch);
  } catch (error) {
    setState({ busy: null, banner: errorBanner(error, "导入当前配置失败") });
  }
}
