import { describe, expect, test } from "bun:test";

import {
  disclosureFor,
  INLINE_OPEN_LIMIT,
  PREVIEW_LIMIT,
  previewText,
} from "../src/chat/disclosure";

const LONG_SUFFIX = "\nUSER-SUFFIX-9F3A";
const LONG_TEXT = `${"U".repeat(33 * 1024)}${LONG_SUFFIX}`;

describe("disclosureFor", () => {
  test("user and assistant text stays complete with no summary", () => {
    for (const role of ["user", "assistant"]) {
      const disclosure = disclosureFor(role, {
        type: "text",
        text: LONG_TEXT,
      });
      expect(disclosure.mode).toBe("full");
      expect(disclosure.summary).toBe("");
      expect(disclosure.body).toBe(LONG_TEXT);
      expect(disclosure.body.endsWith(LONG_SUFFIX)).toBe(true);
    }
  });

  test("user reasoning is collapsible instead of borrowing the text rule", () => {
    const disclosure = disclosureFor("user", {
      type: "reasoning",
      text: "short reasoning",
    });
    expect(disclosure.mode).toBe("collapsible");
    expect(disclosure.open).toBe(true);
  });

  test("tool parts always start collapsed and keep the complete body", () => {
    const toolText = `工具：read\n状态：completed\n输出：${"T".repeat(32 * 1024)}\nTOOL-SUFFIX-4D8E`;
    const disclosure = disclosureFor("assistant", {
      type: "tool",
      text: toolText,
    });
    expect(disclosure.mode).toBe("collapsible");
    expect(disclosure.open).toBe(false);
    expect(disclosure.body).toBe(toolText);
    expect(disclosure.body.endsWith("TOOL-SUFFIX-4D8E")).toBe(true);
    expect(disclosure.summary).toContain("工具：read");
    expect(disclosure.summary).toContain("状态：completed");
    expect(Array.from(disclosure.summary).length).toBeLessThanOrEqual(
      PREVIEW_LIMIT + 1,
    );
  });

  test("small reasoning opens and large reasoning closes at the threshold", () => {
    const small = disclosureFor("assistant", {
      type: "reasoning",
      text: "a".repeat(INLINE_OPEN_LIMIT),
    });
    const large = disclosureFor("assistant", {
      type: "reasoning",
      text: "a".repeat(INLINE_OPEN_LIMIT + 1),
    });
    expect(small.open).toBe(true);
    expect(large.open).toBe(false);
    expect(small.body.length).toBe(INLINE_OPEN_LIMIT);
    expect(large.body.length).toBe(INLINE_OPEN_LIMIT + 1);
  });

  test("other part types follow the reasoning threshold", () => {
    const small = disclosureFor("assistant", {
      type: "file",
      text: "short attachment note",
    });
    const large = disclosureFor("assistant", {
      type: "file",
      text: "b".repeat(INLINE_OPEN_LIMIT + 1),
    });
    expect(small.mode).toBe("collapsible");
    expect(small.open).toBe(true);
    expect(large.open).toBe(false);
  });

  test("empty parts render an empty body for the view fallback", () => {
    const tool = disclosureFor("assistant", { type: "tool", text: "" });
    const reasoning = disclosureFor("assistant", {
      type: "reasoning",
      text: "",
    });
    expect(tool.body).toBe("");
    expect(tool.open).toBe(false);
    expect(reasoning.body).toBe("");
    expect(reasoning.open).toBe(true);
  });
});

describe("previewText", () => {
  test("collapses whitespace into one readable summary line", () => {
    expect(previewText("工具：read\n状态：completed\n\n输出：ok")).toBe(
      "工具：read 状态：completed 输出：ok",
    );
  });

  test("keeps short text unchanged and caps long text at the limit", () => {
    expect(previewText("short")).toBe("short");
    expect(previewText("a".repeat(PREVIEW_LIMIT))).toBe("a".repeat(PREVIEW_LIMIT));

    const capped = previewText("a".repeat(PREVIEW_LIMIT + 1));
    expect(Array.from(capped).length).toBe(PREVIEW_LIMIT + 1);
    expect(capped.endsWith("…")).toBe(true);
  });

  test("never splits a surrogate pair", () => {
    const capped = previewText("😀".repeat(PREVIEW_LIMIT + 1));
    expect(Array.from(capped).length).toBe(PREVIEW_LIMIT + 1);
    expect(capped.endsWith("…")).toBe(true);
    expect(capped.includes("\uFFFD")).toBe(false);
    expect(capped).toBe(`${"😀".repeat(PREVIEW_LIMIT)}…`);
  });
});
