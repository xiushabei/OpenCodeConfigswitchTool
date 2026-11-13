import { patchOmoModelCommand } from "../api";
import type { OmoSection } from "../models/types";
import { getState, setState } from "../state/store";
import { bumpSelectionToken, updateDraft } from "./snapshot";
import { errorBanner } from "./shared";

export type PatchOutcome = "applied" | "stale" | "failed" | "unavailable";

/**
 * Patches one model assignment through the backend so integer values and
 * unknown fields survive without a frontend JSON round-trip.
 */
export async function patchOmoModelField(
  section: OmoSection,
  key: string,
  model: string | null,
): Promise<PatchOutcome> {
  const state = getState();
  const draft = state.draft;
  if (draft === null || draft.target !== "omo") {
    return "unavailable";
  }
  const baseContent = draft.content;
  const baseToken = state.selectionToken;
  try {
    const next = await patchOmoModelCommand(
      baseContent,
      section,
      key,
      model,
    );
    const current = getState();
    if (
      current.draft === null ||
      current.draft.target !== "omo" ||
      current.selectionToken !== baseToken ||
      current.draft.content !== baseContent
    ) {
      return "stale";
    }
    updateDraft({ content: next });
    bumpSelectionToken();
    return "applied";
  } catch (error) {
    setState({ banner: errorBanner(error, "更新模型字段失败") });
    return "failed";
  }
}
