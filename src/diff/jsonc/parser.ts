import { canonicalNumber, matchNumber } from "./number";
import { fail } from "./syntax";
import type { JsoncNode } from "./types";

const MAX_DEPTH = 256;

export class Parser {
  private index: number;
  private readonly text: string;

  constructor(text: string) {
    this.text = text;
    this.index = text.charCodeAt(0) === 0xfeff ? 1 : 0;
  }

  parse(): JsoncNode {
    this.skipTrivia();
    const value = this.parseValue(0);
    this.skipTrivia();
    if (this.index < this.text.length) {
      fail("JSON 结束后存在多余内容。", this.index);
    }
    return value;
  }

  private skipTrivia(): void {
    const text = this.text;
    while (this.index < text.length) {
      const char = text[this.index] ?? "";
      if (
        char === " " ||
        char === "\t" ||
        char === "\n" ||
        char === "\r"
      ) {
        this.index += 1;
        continue;
      }
      if (char !== "/") {
        return;
      }
      const next = text[this.index + 1] ?? "";
      if (next === "/") {
        this.index += 2;
        while (this.index < text.length) {
          const content = text[this.index] ?? "";
          this.index += 1;
          if (content === "\n" || content === "\r") {
            break;
          }
        }
        continue;
      }
      if (next === "*") {
        const start = this.index;
        this.index += 2;
        let closed = false;
        while (this.index < text.length) {
          if (
            text[this.index] === "*" &&
            text[this.index + 1] === "/"
          ) {
            this.index += 2;
            closed = true;
            break;
          }
          this.index += 1;
        }
        if (!closed) {
          fail("块注释未闭合。", start);
        }
        continue;
      }
      return;
    }
  }

  private parseValue(depth: number): JsoncNode {
    if (depth > MAX_DEPTH) {
      fail("JSON 嵌套层级过深。", this.index);
    }
    this.skipTrivia();
    if (this.index >= this.text.length) {
      fail("JSON 意外结束。", this.index);
    }
    const char = this.text[this.index] ?? "";
    if (char === "{") {
      return this.parseObject(depth + 1);
    }
    if (char === "[") {
      return this.parseArray(depth + 1);
    }
    if (char === '"') {
      return { kind: "string", value: this.parseString() };
    }
    if (char === "t") {
      this.expectWord("true");
      return { kind: "boolean", value: true };
    }
    if (char === "f") {
      this.expectWord("false");
      return { kind: "boolean", value: false };
    }
    if (char === "n") {
      this.expectWord("null");
      return { kind: "null" };
    }
    if (char === "-" || (char >= "0" && char <= "9")) {
      return { kind: "number", canonical: this.parseNumber() };
    }
    fail(`意外的字符 ${JSON.stringify(char)}。`, this.index);
  }

  private expectWord(word: string): void {
    if (this.text.slice(this.index, this.index + word.length) !== word) {
      fail(`应为 ${word}。`, this.index);
    }
    this.index += word.length;
  }

  private parseNumber(): string {
    const start = this.index;
    const token = matchNumber(this.text, this.index);
    if (token === null) {
      fail("数字格式无效。", start);
    }
    this.index = token.end;
    return canonicalNumber(token.raw);
  }

  private parseString(): string {
    const start = this.index;
    this.index += 1;
    let escaped = false;
    while (this.index < this.text.length) {
      const char = this.text[this.index] ?? "";
      if (escaped) {
        escaped = false;
        this.index += 1;
        continue;
      }
      if (char === "\\") {
        escaped = true;
        this.index += 1;
        continue;
      }
      if (char === '"') {
        this.index += 1;
        const raw = this.text.slice(start, this.index);
        try {
          const value: unknown = JSON.parse(raw);
          if (typeof value !== "string") {
            fail("JSON 字符串无效。", start);
          }
          return value;
        } catch {
          return fail("JSON 字符串转义无效。", start);
        }
      }
      const code = this.text.charCodeAt(this.index);
      if (code < 0x20) {
        fail("JSON 字符串包含未转义的控制字符。", this.index);
      }
      this.index += 1;
    }
    return fail("字符串未闭合。", start);
  }

  private parseObject(depth: number): JsoncNode {
    const start = this.index;
    this.index += 1;
    const entries = new Map<string, JsoncNode>();
    for (;;) {
      this.skipTrivia();
      if (this.index >= this.text.length) {
        fail("对象未闭合。", start);
      }
      if (this.text[this.index] === "}") {
        this.index += 1;
        break;
      }
      if (this.text[this.index] !== '"') {
        fail("对象键必须是字符串。", this.index);
      }
      const key = this.parseString();
      this.skipTrivia();
      if (this.text[this.index] !== ":") {
        fail("对象键后应为冒号。", this.index);
      }
      this.index += 1;
      const value = this.parseValue(depth);
      entries.set(key, value);
      this.skipTrivia();
      const separator = this.text[this.index] ?? "";
      if (separator === ",") {
        this.index += 1;
        this.skipTrivia();
        if (this.text[this.index] === "}") {
          this.index += 1;
          break;
        }
        continue;
      }
      if (separator === "}") {
        this.index += 1;
        break;
      }
      fail("对象成员之间应为逗号或右花括号。", this.index);
    }
    return {
      kind: "object",
      entries: [...entries.entries()]
        .sort(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0))
        .map(([key, value]) => ({ key, value })),
    };
  }

  private parseArray(depth: number): JsoncNode {
    const start = this.index;
    this.index += 1;
    const items: JsoncNode[] = [];
    for (;;) {
      this.skipTrivia();
      if (this.index >= this.text.length) {
        fail("数组未闭合。", start);
      }
      if (this.text[this.index] === "]") {
        this.index += 1;
        break;
      }
      items.push(this.parseValue(depth));
      this.skipTrivia();
      const separator = this.text[this.index] ?? "";
      if (separator === ",") {
        this.index += 1;
        this.skipTrivia();
        if (this.text[this.index] === "]") {
          this.index += 1;
          break;
        }
        continue;
      }
      if (separator === "]") {
        this.index += 1;
        break;
      }
      fail("数组元素之间应为逗号或右方括号。", this.index);
    }
    return { kind: "array", items };
  }
}
