import type { JsoncNode } from "./types";

function scalarText(node: JsoncNode): string {
  switch (node.kind) {
    case "null":
      return "null";
    case "boolean":
      return node.value ? "true" : "false";
    case "number":
      return node.canonical;
    case "string":
      return JSON.stringify(node.value);
    case "array":
      return "[]";
    case "object":
      return "{}";
  }
}

function keySegment(key: string): string {
  return /^[A-Za-z_][A-Za-z0-9_]*$/.test(key)
    ? `.${key}`
    : `[${JSON.stringify(key)}]`;
}

function pushField(
  node: JsoncNode,
  path: string,
  out: string[],
): void {
  if (node.kind === "object") {
    if (node.entries.length === 0) {
      out.push(`${path} = {}`);
      return;
    }
    for (const entry of node.entries) {
      pushField(entry.value, `${path}${keySegment(entry.key)}`, out);
    }
    return;
  }
  if (node.kind === "array") {
    if (node.items.length === 0) {
      out.push(`${path} = []`);
      return;
    }
    for (const [index, item] of node.items.entries()) {
      pushField(item, `${path}[${index}]`, out);
    }
    return;
  }
  out.push(`${path} = ${scalarText(node)}`);
}

/**
 * One line per leaf value, in stable object-key order and array-index order.
 *
 * This is the display form for a semantic diff: no braces, commas or indentation
 * that could create formatting-only rows. `$` is the document root.
 */
export function jsoncFieldLines(node: JsoncNode): string[] {
  const out: string[] = [];
  pushField(node, "$", out);
  return out;
}
