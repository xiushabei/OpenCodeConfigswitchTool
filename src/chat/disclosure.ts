import type { ChatPart } from "./types";

/** Characters kept in a collapsed summary; the complete body stays available. */
export const PREVIEW_LIMIT = 160;

/** Reasoning and other parts open by default at or below this length. */
export const INLINE_OPEN_LIMIT = 2000;

export type DisclosureMode = "full" | "collapsible";

export interface PartDisclosure {
  mode: DisclosureMode;
  /** Initial state of the native details element. */
  open: boolean;
  /** Short, whitespace-normalized text for a collapsible summary. */
  summary: string;
  /** The received text, never shortened by the interface. */
  body: string;
}

function characterCount(text: string): number {
  return Array.from(text).length;
}

/** Collapses whitespace and keeps a bounded, code-point-safe preview. */
export function previewText(text: string, limit: number = PREVIEW_LIMIT): string {
  const collapsed = text.replace(/\s+/gu, " ").trim();
  const characters = Array.from(collapsed);
  if (characters.length <= limit) {
    return collapsed;
  }
  return `${characters.slice(0, limit).join("")}…`;
}

/**
 * Chooses the disclosure treatment for one part.
 *
 * User and assistant text is complete by default. Tool parts always start
 * collapsed behind a summary. Reasoning and other parts start open when small,
 * because hiding a short trace adds a click without saving space.
 */
export function disclosureFor(
  role: string,
  part: Pick<ChatPart, "type" | "text">,
): PartDisclosure {
  if (part.type === "text" && (role === "user" || role === "assistant")) {
    return { mode: "full", open: true, summary: "", body: part.text };
  }
  const open =
    part.type !== "tool" && characterCount(part.text) <= INLINE_OPEN_LIMIT;
  return {
    mode: "collapsible",
    open,
    summary: previewText(part.text),
    body: part.text,
  };
}
