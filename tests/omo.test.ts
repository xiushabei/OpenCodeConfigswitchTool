import { describe, expect, test } from "bun:test";

import {
  assignmentPrefix,
  lookupAssignment,
  readAssignments,
} from "../src/models/omo";

function sectionEntries(
  prefix: string,
  count: number,
): Record<string, { model: string }> {
  return Object.fromEntries(
    Array.from({ length: count }, (_, index) => [
      `${prefix}-${index + 1}`,
      { model: `provider/model-${index + 1}` },
    ]),
  );
}

const WRAPPED_CONTENT = JSON.stringify({
  $schema: "https://example.test/omo.schema.json",
  model: "openai/gpt-5",
  "[opencode]": {
    agents: sectionEntries("agent", 12),
    categories: sectionEntries("category", 8),
    theme: "unknown-wrapper-field",
  },
  agents: { ghost: { model: "ghost/model" } },
});

describe("wrapped OMO assignments", () => {
  test("reads 12 agents and 8 categories from the [opencode] wrapper", () => {
    const read = readAssignments(WRAPPED_CONTENT);

    expect(read.ok).toBe(true);
    if (!read.ok) {
      return;
    }
    expect(read.rows).toHaveLength(20);
    expect(read.rows.filter((row) => row.section === "agents")).toHaveLength(12);
    expect(
      read.rows.filter((row) => row.section === "categories"),
    ).toHaveLength(8);
    expect(read.rows.every((row) => row.wrapped)).toBe(true);
    expect(read.rows).toContainEqual({
      section: "agents",
      key: "agent-1",
      model: "provider/model-1",
      wrapped: true,
    });
    expect(read.rows).toContainEqual({
      section: "categories",
      key: "category-8",
      model: "provider/model-8",
      wrapped: true,
    });
    expect(read.rows.some((row) => row.key === "ghost")).toBe(false);
    expect(assignmentPrefix("agents", true)).toBe("[opencode].agents");
  });

  test("lookupAssignment resolves every kind inside the wrapper", () => {
    const content = JSON.stringify({
      "[opencode]": {
        agents: {
          configured: { model: "openai/gpt-5" },
          bare: { temperature: 0.1 },
          text: "not-an-object",
        },
        categories: {},
      },
      agents: { configured: { model: "flat/ignored" } },
    });

    expect(lookupAssignment(content, "agents", "configured")).toEqual({
      ok: true,
      kind: "has-model",
      wrapped: true,
    });
    expect(lookupAssignment(content, "agents", "bare")).toEqual({
      ok: true,
      kind: "no-model",
      wrapped: true,
    });
    expect(lookupAssignment(content, "agents", "text")).toEqual({
      ok: true,
      kind: "non-object",
      wrapped: true,
    });
    expect(lookupAssignment(content, "agents", "ghost")).toEqual({
      ok: true,
      kind: "missing",
      wrapped: true,
    });
  });

  test("a non-object [opencode] wrapper is an error, never a flat fallback", () => {
    const content = JSON.stringify({
      "[opencode]": [],
      agents: { ghost: { model: "ghost/model" } },
    });

    const read = readAssignments(content);
    expect(read.ok).toBe(false);
    if (read.ok) {
      return;
    }
    expect(read.message).toContain("[opencode]");
    expect(lookupAssignment(content, "agents", "ghost").ok).toBe(false);
  });

  test("flat root assignments are ignored while the wrapper exists", () => {
    const content = JSON.stringify({
      "[opencode]": { agents: {}, categories: {} },
      agents: { ghost: { model: "ghost/model" } },
      categories: { ghost: { model: "ghost/model" } },
    });

    const read = readAssignments(content);
    expect(read.ok).toBe(true);
    if (!read.ok) {
      return;
    }
    expect(read.rows).toEqual([]);
    expect(lookupAssignment(content, "categories", "ghost")).toEqual({
      ok: true,
      kind: "missing",
      wrapped: true,
    });
  });

  test("reads stay strict until JSONC is normalized", () => {
    const content =
      '{"[opencode]": {"agents": {}, // comment\n "categories": {}}}';

    expect(readAssignments(content).ok).toBe(false);
  });
});

describe("legacy flat OMO assignments", () => {
  test("keep reading with unwrapped paths", () => {
    const content = JSON.stringify({
      agents: sectionEntries("agent", 12),
      categories: sectionEntries("category", 8),
    });

    const read = readAssignments(content);
    expect(read.ok).toBe(true);
    if (!read.ok) {
      return;
    }
    expect(read.rows).toHaveLength(20);
    expect(read.rows.every((row) => !row.wrapped)).toBe(true);
    expect(assignmentPrefix("categories", false)).toBe("categories");
    expect(lookupAssignment(content, "agents", "agent-3")).toEqual({
      ok: true,
      kind: "has-model",
      wrapped: false,
    });
  });
});
