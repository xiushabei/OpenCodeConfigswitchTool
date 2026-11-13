import { icon } from "../icons/icons";
import type { Filter, UiState } from "../state/store";
import {
  activePresetIds,
  filteredPresets,
  presetCountFor,
} from "../state/selectors";
import { targetLabel } from "../format";
import type { Preset } from "../types";
import { badge, button } from "./controls";
import { create, replaceChildren } from "./dom";

export interface PresetListView {
  element: HTMLElement;
  update(state: UiState): void;
}

export interface PresetListOptions {
  onFilterChange(filter: Filter): void;
  onSelect(id: string): void;
  onNewPreset(): void;
}

interface FilterSpec {
  filter: Filter;
  label: string;
}

const FILTERS: readonly FilterSpec[] = [
  { filter: "all", label: "全部" },
  { filter: "opencode", label: "OpenCode" },
  { filter: "omo", label: "OMO" },
];

export function createPresetList(options: PresetListOptions): PresetListView {
  function presetRow(
    preset: Preset,
    state: UiState,
    activeIds: ReadonlySet<string>,
  ): HTMLButtonElement {
    const selected = state.selectedId === preset.id;
    const active = activeIds.has(preset.id);

    const body = create("span", "preset-item__body", [
      create("span", "preset-item__name truncate", [preset.name]),
    ]);
    if (preset.description.trim().length > 0) {
      body.append(
        create("span", "preset-item__desc truncate", [preset.description]),
      );
    }

    const meta = create("span", "preset-item__meta", [
      badge(targetLabel(preset.target), "target"),
    ]);
    if (active) {
      meta.append(badge("当前生效", "active", "check_circle"));
    }

    const row = create("button", "preset-item", [body, meta]);
    row.type = "button";
    row.setAttribute("aria-pressed", String(selected));
    row.setAttribute("data-preset-id", preset.id);
    row.title = active
      ? `${preset.name}（当前生效）`
      : `${preset.name}（${targetLabel(preset.target)}）`;
    row.addEventListener("click", () => {
      if (state.busy === null && state.status === "ready") {
        options.onSelect(preset.id);
      }
    });
    return row;
  }
  const count = create("span", "rail__count", ["0 个"]);
  const head = create("div", "rail__head", [
    create("h2", "rail__title", ["预设"]),
    count,
  ]);

  const filterButtons = new Map<Filter, HTMLButtonElement>();
  const filterCounts = new Map<Filter, HTMLSpanElement>();
  const filters = create("div", "segmented");
  filters.setAttribute("role", "group");
  filters.setAttribute("aria-label", "按目标筛选预设");

  for (const spec of FILTERS) {
    const filterCount = create("span", "segmented__count", ["0"]);
    const check = create("span", "segmented__check", [icon("check", 12)]);
    const item = create("button", "segmented__item", [
      check,
      create("span", "segmented__label", [spec.label]),
      filterCount,
    ]);
    item.type = "button";
    item.setAttribute("aria-pressed", "false");
    item.addEventListener("click", () => {
      options.onFilterChange(spec.filter);
    });
    filterButtons.set(spec.filter, item);
    filterCounts.set(spec.filter, filterCount);
    filters.append(item);
  }

  const list = create("div", "rail__list");
  list.setAttribute("role", "group");
  list.setAttribute("aria-label", "预设列表");

  const newButton = button("新建预设", "tonal", "add");
  newButton.classList.add("btn--full");
  newButton.addEventListener("click", options.onNewPreset);
  const foot = create("div", "rail__foot", [newButton]);

  const filterWrap = create("div", "rail__filters", [filters]);
  const element = create("aside", "rail", [head, filterWrap, list, foot]);
  let signature = "";

  function update(state: UiState): void {
    const presets = filteredPresets(state);
    const activeIds = activePresetIds(state);
    const disabled = state.status !== "ready" || state.busy !== null;

    count.textContent = `${presets.length} 个`;
    const allCount = state.snapshot?.presets.length ?? 0;
    const opencodeCount = presetCountFor(state, "opencode");
    const omoCount = presetCountFor(state, "omo");
    filterCounts.get("all")?.replaceChildren(String(allCount));
    filterCounts.get("opencode")?.replaceChildren(String(opencodeCount));
    filterCounts.get("omo")?.replaceChildren(String(omoCount));
    for (const [filter, item] of filterButtons) {
      item.setAttribute("aria-pressed", String(state.filter === filter));
      item.disabled = state.status !== "ready";
    }
    newButton.disabled = disabled;
    newButton.setAttribute(
      "title",
      disabled ? "桌面运行时不可用或正在处理" : "新建预设",
    );

    const next = JSON.stringify({
      filter: state.filter,
      selectedId: state.selectedId,
      status: state.status,
      busy: state.busy,
      list: presets.map((preset) => [
        preset.id,
        preset.name,
        preset.description,
        preset.target,
        activeIds.has(preset.id),
      ]),
    });
    if (next === signature) {
      return;
    }
    signature = next;

    if (presets.length === 0) {
      const message =
        state.status === "loading"
          ? "正在读取预设…"
          : state.status === "desktop-required"
            ? "需要桌面运行时才能读取预设。"
            : state.status === "error"
              ? "预设读取失败。"
              : state.filter === "all"
                ? "还没有预设，先新建一个。"
                : `此目标下还没有预设。`;
      replaceChildren(list, [create("p", "rail-empty", [message])]);
      return;
    }

    const hadFocus = list.contains(document.activeElement);
    const rows = presets.map((preset) => presetRow(preset, state, activeIds));
    replaceChildren(list, rows);
    if (hadFocus) {
      const selected = list.querySelector('[aria-pressed="true"]');
      if (selected instanceof HTMLElement) {
        selected.focus();
      }
    }
  }

  return { element, update };
}
