import { describe, expect, test } from "bun:test";

import {
  canonicalConfig,
  configsEquivalent,
  diffConfigs,
  diffLines,
  parseJsonc,
} from "../src/diff";

describe("diffLines", () => {
  test("reports an inserted line and keeps surrounding context", () => {
    const result = diffLines(
      "alpha\nbeta\ngamma\n",
      "alpha\ninserted\nbeta\ngamma\n",
    );

    expect(result.identical).toBe(false);
    expect(result.added).toBe(1);
    expect(result.removed).toBe(0);
    expect(result.lines.map((line) => [line.kind, line.text])).toEqual([
      ["same", "alpha"],
      ["add", "inserted"],
      ["same", "beta"],
      ["same", "gamma"],
      ["same", ""],
    ]);
  });

  test("reports a removed line without inventing an addition", () => {
    const result = diffLines(
      "alpha\nbeta\ngamma\n",
      "alpha\ngamma\n",
    );

    expect(result.identical).toBe(false);
    expect(result.added).toBe(0);
    expect(result.removed).toBe(1);
    expect(result.lines.map((line) => [line.kind, line.text])).toEqual([
      ["same", "alpha"],
      ["remove", "beta"],
      ["same", "gamma"],
      ["same", ""],
    ]);
  });

  test("keeps multi-line insert and remove rows in order", () => {
    const result = diffLines(
      "head\nkeep\nold-a\nold-b\ntail\n",
      "head\nnew-a\nnew-b\nkeep\ntail\n",
    );

    expect(result.added).toBe(2);
    expect(result.removed).toBe(2);
    expect(
      result.lines
        .filter((line) => line.kind !== "same")
        .map((line) => `${line.kind}:${line.text}`),
    ).toEqual([
      "add:new-a",
      "add:new-b",
      "remove:old-a",
      "remove:old-b",
    ]);
  });

  test("shows both sides of a compact JSON field change", () => {
    const result = diffLines(
      '{"a":1,"b":2}',
      '{"a":1,"b":3}',
    );

    expect(result.identical).toBe(false);
    expect(result.added).toBe(1);
    expect(result.removed).toBe(1);
    expect(result.lines.map((line) => line.kind)).toEqual([
      "remove",
      "add",
    ]);
    expect(result.lines[0]?.text).toBe('{"a":1,"b":2}');
    expect(result.lines[1]?.text).toBe('{"a":1,"b":3}');
  });
});

describe("JSONC semantic diff", () => {
  const target = `{
  // current target keeps comments and its own formatting
  "theme": "opencode",
  "model": "anthropic/claude-sonnet",
  "items": [1, 2, 3,],
}`;

  test("formatting, comments, trailing commas and key order are not rows", () => {
    const draft =
      '{"items":[1,2,3],"model":"anthropic/claude-sonnet","theme":"opencode"}';
    const diff = diffConfigs(target, draft);

    expect(diff.mode).toBe("semantic");
    expect(diff.semanticEqual).toBe(true);
    expect(diff.rawIdentical).toBe(false);
    expect(diff.result.identical).toBe(true);
    expect(diff.result.added).toBe(0);
    expect(diff.result.removed).toBe(0);
    expect(diff.result.lines).toHaveLength(0);
    expect(configsEquivalent(target, draft)).toBe(true);
  });

  test("a real key edit shows only the changed field", () => {
    const draft =
      '{"items":[1,2,3],"model":"openai/gpt-5.2","theme":"opencode"}';
    const diff = diffConfigs(target, draft);
    const changed = diff.result.lines
      .filter((line) => line.kind !== "same")
      .map((line) => `${line.kind}:${line.text}`);

    expect(diff.mode).toBe("semantic");
    expect(diff.semanticEqual).toBe(false);
    expect(diff.result.added).toBe(1);
    expect(diff.result.removed).toBe(1);
    expect(changed).toEqual([
      'remove:$.model = "anthropic/claude-sonnet"',
      'add:$.model = "openai/gpt-5.2"',
    ]);
    expect(diff.result.lines.length).toBeLessThan(10);
    expect(diff.result.lines.every((line) => line.oldLine === null)).toBe(true);
    expect(diff.result.lines.every((line) => line.newLine === null)).toBe(true);
  });

  test("array order stays meaningful", () => {
    const diff = diffConfigs(
      '{"items":[1,2,3]}',
      '{"items":[3,2,1]}',
    );

    expect(diff.mode).toBe("semantic");
    expect(diff.semanticEqual).toBe(false);
    expect(diff.result.added).toBeGreaterThan(0);
    expect(diff.result.removed).toBeGreaterThan(0);
    expect(
      diff.result.lines
        .filter((line) => line.kind !== "same")
        .map((line) => line.text),
    ).toContain("$.items[0] = 3");
  });

  test("integers beyond 2^53 stay exact and different", () => {
    const exact = '{"big": 9007199254740993, "keep": true}';
    const reordered = '{"keep": true, "big": 9007199254740993}';
    const rounded = '{"big": 9007199254740992, "keep": true}';

    expect(configsEquivalent(exact, reordered)).toBe(true);
    expect(configsEquivalent(exact, rounded)).toBe(false);

    const diff = diffConfigs(exact, rounded);
    const changed = diff.result.lines
      .filter((line) => line.kind !== "same")
      .map((line) => line.text);
    expect(changed).toContain("$.big = 9007199254740993");
    expect(changed).toContain("$.big = 9007199254740992");
  });

  test("equivalent number spellings compare equal without Number rounding", () => {
    const left = '{"n": 1.0, "z": -0.0, "e": 1e3}';
    const right = '{"e": 1000, "z": 0, "n": 1}';

    expect(configsEquivalent(left, right)).toBe(true);
  });

  test("comments inside strings are not stripped", () => {
    const left = '{"url": "http://example.test//path", "s": "\\u0041"}';
    const right = '{"s": "A", "url": "http://example.test//path"}';

    expect(configsEquivalent(left, right)).toBe(true);
  });

  test("a leading BOM and trailing commas are accepted as JSONC", () => {
    expect(configsEquivalent('\uFEFF{"a": 1,}', '{"a": 1}')).toBe(true);
  });
});

describe("invalid JSON fallback", () => {
  test("a broken target uses the raw line diff and reports syntax", () => {
    const diff = diffConfigs('{"a": 1, /* unterminated', '{"a": 1}');

    expect(diff.mode).toBe("raw");
    expect(diff.semanticEqual).toBe(false);
    expect(diff.targetError).not.toBeNull();
    expect(diff.targetError?.line).toBe(1);
    expect(diff.targetError?.column).toBe(10);
    expect(diff.draftError).toBeNull();
    expect(diff.result.added).toBeGreaterThan(0);
    expect(diff.result.removed).toBeGreaterThan(0);
  });

  test("identical invalid text is never reported as semantically equal", () => {
    const broken = '{"a": 01}';
    const diff = diffConfigs(broken, broken);

    expect(diff.mode).toBe("raw");
    expect(diff.rawIdentical).toBe(true);
    expect(diff.result.identical).toBe(true);
    expect(diff.semanticEqual).toBe(false);
    expect(diff.targetError).not.toBeNull();
    expect(diff.draftError).not.toBeNull();
    expect(configsEquivalent(broken, broken)).toBe(false);
  });

  test("a non-object document is rejected instead of compared", () => {
    const diff = diffConfigs("[1,2,3]", "[1,2,3]");

    expect(diff.mode).toBe("raw");
    expect(diff.semanticEqual).toBe(false);
    expect(diff.targetError?.message).toContain("JSON 对象");
  });

  test("comments between tokens cannot join separate values", () => {
    const parsed = parseJsonc('{"a": 1/* comment */2}');
    expect(parsed.ok).toBe(false);

    const canonical = canonicalConfig('{"a": 1}');
    expect(canonical.ok).toBe(true);
    if (canonical.ok) {
      expect(canonical.fields).toBe("$.a = 1");
    }
  });
});
