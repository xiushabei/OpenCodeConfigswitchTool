export type Child = Node | string;

export function create<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  className: string = "",
  children: readonly Child[] = [],
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  if (className.length > 0) {
    node.className = className;
  }
  if (children.length > 0) {
    node.append(...children);
  }
  return node;
}

export function replaceChildren(target: Element, children: readonly Child[]): void {
  target.replaceChildren(...children);
}

export function requireMount(id: string): HTMLElement {
  const node = document.getElementById(id);
  if (node === null) {
    throw new Error(`缺少挂载节点 #${id}`);
  }
  return node;
}
