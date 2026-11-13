import { icon } from "../icons/icons";
import { badge } from "../ui/controls";
import { create, replaceChildren } from "../ui/dom";
import { disclosureFor } from "./disclosure";
import { formatTime, sessionTitle } from "./format";
import type { ChatMessage, ChatPart, ChatReaderData } from "./types";

export interface ChatReaderView {
  element: HTMLElement;
  showPrompt(): void;
  showLoading(title: string): void;
  showError(message: string): void;
  showReader(data: ChatReaderData): void;
}

const ROLE_LABELS: Record<string, string> = {
  user: "用户",
  assistant: "助手",
  system: "系统",
  tool: "工具",
};

const PART_LABELS: Record<string, string> = {
  text: "文本",
  reasoning: "推理",
  tool: "工具",
};

function roleLabel(role: string): string {
  return ROLE_LABELS[role] ?? role;
}

function partLabel(type: string): string {
  return PART_LABELS[type] ?? `其他（${type}）`;
}

function partBody(text: string): HTMLElement {
  return create("pre", "chat-part__text", [
    text.length > 0 ? text : "（空内容）",
  ]);
}

function partCard(role: string, part: ChatPart): HTMLElement {
  const disclosure = disclosureFor(role, part);
  const head = create("div", "chat-part__head", [
    create("span", "chat-part__label", [partLabel(part.type)]),
  ]);
  if (part.truncated) {
    const flag = badge("解析不完整", "error");
    flag.title = "后端读取该片段时无法完整解析。";
    head.append(flag);
  }
  const className =
    part.type === "tool" ? "chat-part chat-part--tool" : "chat-part";
  if (disclosure.mode === "full") {
    return create("section", className, [head, partBody(disclosure.body)]);
  }
  const summary = create("summary", "chat-part__summary", [
    create("span", "chat-part__toggle chat-part__toggle--closed", ["展开"]),
    create("span", "chat-part__toggle chat-part__toggle--open", ["收回"]),
    create("span", "chat-part__preview", [
      disclosure.summary.length > 0 ? disclosure.summary : "（空内容）",
    ]),
  ]);
  summary.title = "展开或收回完整内容";
  const details = create("details", "chat-part__details", [
    summary,
    partBody(disclosure.body),
  ]);
  details.open = disclosure.open;
  return create("section", className, [head, details]);
}

function messageCard(message: ChatMessage): HTMLElement {
  const head = create("header", "chat-msg__head", [
    create("span", "chat-msg__role", [roleLabel(message.role)]),
    create("span", "chat-msg__time mono", [formatTime(message.created ?? 0)]),
  ]);
  const parts = message.parts.map((part) => partCard(message.role, part));
  return create("article", "chat-msg", [head, ...parts]);
}

function promptState(
  iconName: "description" | "error",
  title: string,
  text: string,
  tone: "neutral" | "error",
): HTMLElement {
  const box = create(
    "div",
    tone === "error"
      ? "empty-state empty-state--inline chat-empty--error"
      : "empty-state empty-state--inline",
    [
      create("span", "empty-state__icon", [icon(iconName, 26)]),
      create("p", "empty-state__title", [title]),
      create("p", "empty-state__text", [text]),
    ],
  );
  return box;
}

export function createChatReaderPane(): ChatReaderView {
  const root = create("section", "chat-reader");
  root.setAttribute("aria-label", "会话内容");

  function showPrompt(): void {
    replaceChildren(root, [
      promptState(
        "description",
        "选择会话",
        "从左侧选择一个会话查看消息内容。读取为只读操作，不会修改数据库。",
        "neutral",
      ),
    ]);
    root.scrollTop = 0;
  }

  function showLoading(title: string): void {
    replaceChildren(root, [
      create("div", "chat-reader__head", [
        create("h3", "chat-reader__title", [sessionTitle(title)]),
        create("p", "chat-reader__meta", ["正在读取会话…"]),
      ]),
      create("p", "chat-list__empty", ["正在读取…"]),
    ]);
    root.scrollTop = 0;
  }

  function showError(message: string): void {
    replaceChildren(root, [
      promptState("error", "读取失败", message, "error"),
    ]);
    root.scrollTop = 0;
  }

  function showReader(data: ChatReaderData): void {
    const kids: HTMLElement[] = [
      create("div", "chat-reader__head", [
        create("h3", "chat-reader__title", [sessionTitle(data.title)]),
        create("p", "mono muted", [
          `${data.id} · ${data.messages.length} 条消息`,
        ]),
      ]),
    ];

    const warnings = data.warnings.filter((line) => line.trim().length > 0);
    if (warnings.length > 0) {
      kids.push(create("p", "chat-notice", [warnings.join("\n")]));
    }
    if (data.truncated) {
      kids.push(
        create("p", "chat-notice", [
          "该会话有部分数据无法完整解析，以下内容可能不完整。",
        ]),
      );
    }

    if (data.messages.length === 0) {
      kids.push(create("p", "chat-list__empty", ["该会话没有可显示的消息。"]));
    } else {
      kids.push(
        create("div", "chat-messages", data.messages.map(messageCard)),
      );
    }
    replaceChildren(root, kids);
    root.scrollTop = 0;
  }

  showPrompt();
  return { element: root, showPrompt, showLoading, showError, showReader };
}
