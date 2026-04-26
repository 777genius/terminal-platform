import type { TerminalTabStripItemControlState } from "./terminal-tab-strip-controls.js";

export type TerminalTabStripKeyboardIntent =
  | {
    kind: "none";
  }
  | {
    kind: "focus-tab";
    itemKey: string;
    tabId: string;
  }
  | {
    kind: "close-tab";
    itemKey: string;
    tabId: string;
  };

export interface TerminalTabStripKeyboardInput {
  currentItemKey: string;
  key: string;
}

export function resolveTerminalTabStripKeyboardIntent(
  tabs: readonly TerminalTabStripItemControlState[],
  input: TerminalTabStripKeyboardInput,
): TerminalTabStripKeyboardIntent {
  if (tabs.length === 0) {
    return { kind: "none" };
  }

  const currentIndex = resolveCurrentTabIndex(tabs, input.currentItemKey);
  const currentTab = currentIndex >= 0 ? tabs[currentIndex] : null;

  if ((input.key === "Delete" || input.key === "Backspace") && currentTab?.canClose) {
    return {
      kind: "close-tab",
      itemKey: currentTab.itemKey,
      tabId: currentTab.tabId,
    };
  }

  const targetIndex = resolveTargetIndex(tabs, currentIndex, input.key);
  if (targetIndex === null || targetIndex === currentIndex) {
    return { kind: "none" };
  }

  const targetTab = tabs[targetIndex];
  if (!targetTab?.canFocus) {
    return { kind: "none" };
  }

  return {
    kind: "focus-tab",
    itemKey: targetTab.itemKey,
    tabId: targetTab.tabId,
  };
}

function resolveCurrentTabIndex(
  tabs: readonly TerminalTabStripItemControlState[],
  currentItemKey: string,
): number {
  const explicitIndex = tabs.findIndex((tab) => tab.itemKey === currentItemKey);
  if (explicitIndex >= 0) {
    return explicitIndex;
  }

  const activeIndex = tabs.findIndex((tab) => tab.active);
  if (activeIndex >= 0) {
    return activeIndex;
  }

  return tabs.findIndex((tab) => tab.canFocus);
}

function resolveTargetIndex(
  tabs: readonly TerminalTabStripItemControlState[],
  currentIndex: number,
  key: string,
): number | null {
  switch (key) {
    case "ArrowLeft":
    case "ArrowUp":
      return resolveRelativeFocusableIndex(tabs, currentIndex, -1);
    case "ArrowRight":
    case "ArrowDown":
      return resolveRelativeFocusableIndex(tabs, currentIndex, 1);
    case "Home":
      return tabs.findIndex((tab) => tab.canFocus);
    case "End":
      return findLastIndex(tabs, (tab) => tab.canFocus);
    default:
      return null;
  }
}

function resolveRelativeFocusableIndex(
  tabs: readonly TerminalTabStripItemControlState[],
  currentIndex: number,
  delta: -1 | 1,
): number | null {
  const focusableCount = tabs.filter((tab) => tab.canFocus).length;
  if (focusableCount <= 1) {
    return null;
  }

  const startIndex = currentIndex >= 0 ? currentIndex : delta > 0 ? -1 : 0;
  for (let step = 1; step <= tabs.length; step += 1) {
    const index = (startIndex + (step * delta) + tabs.length) % tabs.length;
    if (tabs[index]?.canFocus) {
      return index;
    }
  }

  return null;
}

function findLastIndex<T>(items: readonly T[], predicate: (item: T) => boolean): number {
  for (let index = items.length - 1; index >= 0; index -= 1) {
    if (predicate(items[index] as T)) {
      return index;
    }
  }

  return -1;
}
