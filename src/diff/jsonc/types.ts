export interface JsoncSyntaxError {
  readonly message: string;
  readonly index: number;
  readonly line: number;
  readonly column: number;
}

export type JsoncNode =
  | { readonly kind: "null" }
  | { readonly kind: "boolean"; readonly value: boolean }
  | { readonly kind: "number"; readonly canonical: string }
  | { readonly kind: "string"; readonly value: string }
  | { readonly kind: "array"; readonly items: readonly JsoncNode[] }
  | { readonly kind: "object"; readonly entries: readonly JsoncEntry[] };

export interface JsoncEntry {
  readonly key: string;
  readonly value: JsoncNode;
}

export type JsoncResult =
  | { readonly ok: true; readonly value: JsoncNode }
  | { readonly ok: false; readonly error: JsoncSyntaxError };

export function isJsoncObject(
  node: JsoncNode,
): node is Extract<JsoncNode, { kind: "object" }> {
  return node.kind === "object";
}
