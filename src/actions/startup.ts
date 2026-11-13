import { fetchSnapshot, restoreStartupCommand } from "../api";
import { getState, setState } from "../state/store";
import { errorMessage } from "../tauri";
import { errorBanner, selectionFor } from "./shared";

export async function restoreStartup(): Promise<void> {
  const state = getState();
  setState({ busy: "restore", banner: null });
  try {
    const result = await restoreStartupCommand();
    setState({
      snapshot: result.snapshot,
      busy: null,
      ...selectionFor(result.snapshot, state.selectedId),
      tab: "edit",
      selectionToken: getState().selectionToken + 1,
      banner: {
        tone: "success",
        message: result.message,
        detail: result.backup === null ? null : `备份：${result.backup}`,
      },
    });
  } catch (error) {
    const original = errorBanner(error, "还原至原配置失败");
    try {
      const snapshot = await fetchSnapshot();
      setState({
        snapshot,
        busy: null,
        ...selectionFor(snapshot, state.selectedId),
        tab: "edit",
        selectionToken: getState().selectionToken + 1,
        banner: original,
      });
    } catch (refreshError) {
      setState({
        busy: null,
        banner: {
          ...original,
          detail: `刷新状态失败：${errorMessage(refreshError)}`,
        },
      });
    }
  }
}
