import { saveSettingsCommand } from "../api";
import { setState } from "../state/store";
import type { Settings } from "../types";
import { applyTheme } from "../ui/theme";
import { errorMessage } from "../tauri";

export async function persistSettings(
  settings: Settings,
): Promise<string | null> {
  setState({ busy: "settings", banner: null });
  try {
    const snapshot = await saveSettingsCommand(settings);
    applyTheme(snapshot.settings.theme);
    setState({ snapshot, busy: null, status: "ready", error: null });
    return null;
  } catch (error) {
    setState({ busy: null });
    return errorMessage(error);
  }
}
