import { describe, expect, it } from "vitest";

import type { TerminalTabStripItemControlState } from "./terminal-tab-strip-controls.js";
import { resolveTerminalTabStripKeyboardIntent } from "./terminal-tab-strip-keyboard-navigation.js";

describe("terminal tab strip keyboard navigation", () => {
  it("wraps arrow navigation across focusable terminal tabs", () => {
    const tabs = createTabs();

    expect(resolveTerminalTabStripKeyboardIntent(tabs, {
      currentItemKey: "tab-2:1",
      key: "ArrowRight",
    })).toEqual({
      kind: "focus-tab",
      itemKey: "tab-3:2",
      tabId: "tab-3",
    });

    expect(resolveTerminalTabStripKeyboardIntent(tabs, {
      currentItemKey: "tab-3:2",
      key: "ArrowRight",
    })).toEqual({
      kind: "focus-tab",
      itemKey: "tab-1:0",
      tabId: "tab-1",
    });

    expect(resolveTerminalTabStripKeyboardIntent(tabs, {
      currentItemKey: "tab-1:0",
      key: "ArrowLeft",
    })).toEqual({
      kind: "focus-tab",
      itemKey: "tab-3:2",
      tabId: "tab-3",
    });
  });

  it("moves to the first and last focusable tab with Home and End", () => {
    const tabs = createTabs();

    expect(resolveTerminalTabStripKeyboardIntent(tabs, {
      currentItemKey: "tab-2:1",
      key: "Home",
    })).toEqual({
      kind: "focus-tab",
      itemKey: "tab-1:0",
      tabId: "tab-1",
    });

    expect(resolveTerminalTabStripKeyboardIntent(tabs, {
      currentItemKey: "tab-2:1",
      key: "End",
    })).toEqual({
      kind: "focus-tab",
      itemKey: "tab-3:2",
      tabId: "tab-3",
    });
  });

  it("skips non-focusable tabs during arrow navigation", () => {
    const tabs = createTabs();
    tabs[1] = createTab({
      itemKey: "tab-2:1",
      tabId: "tab-2",
      canFocus: false,
    });

    expect(resolveTerminalTabStripKeyboardIntent(tabs, {
      currentItemKey: "tab-1:0",
      key: "ArrowRight",
    })).toEqual({
      kind: "focus-tab",
      itemKey: "tab-3:2",
      tabId: "tab-3",
    });
  });

  it("uses the active tab when the focused item key is stale", () => {
    expect(resolveTerminalTabStripKeyboardIntent(createTabs(), {
      currentItemKey: "missing",
      key: "ArrowLeft",
    })).toEqual({
      kind: "focus-tab",
      itemKey: "tab-1:0",
      tabId: "tab-1",
    });
  });

  it("returns a close intent only when the focused tab can close", () => {
    expect(resolveTerminalTabStripKeyboardIntent(createTabs(), {
      currentItemKey: "tab-2:1",
      key: "Delete",
    })).toEqual({
      kind: "close-tab",
      itemKey: "tab-2:1",
      tabId: "tab-2",
    });

    expect(resolveTerminalTabStripKeyboardIntent(createTabs({ canClose: false }), {
      currentItemKey: "tab-2:1",
      key: "Backspace",
    })).toEqual({ kind: "none" });
  });

  it("does not navigate when no alternate focusable tab exists", () => {
    expect(resolveTerminalTabStripKeyboardIntent([
      createTab({ active: true, itemKey: "tab-1:0", tabId: "tab-1" }),
    ], {
      currentItemKey: "tab-1:0",
      key: "ArrowRight",
    })).toEqual({ kind: "none" });
  });
});

function createTabs(overrides: Partial<TerminalTabStripItemControlState> = {}): TerminalTabStripItemControlState[] {
  return [
    createTab({
      itemKey: "tab-1:0",
      tabId: "tab-1",
    }, overrides),
    createTab({
      active: true,
      closeTabIndex: 0,
      itemKey: "tab-2:1",
      tabId: "tab-2",
      tabIndex: 0,
    }, overrides),
    createTab({
      itemKey: "tab-3:2",
      tabId: "tab-3",
    }, overrides),
  ];
}

function createTab(
  values: Partial<TerminalTabStripItemControlState>,
  overrides: Partial<TerminalTabStripItemControlState> = {},
): TerminalTabStripItemControlState {
  return {
    active: false,
    canClose: true,
    canFocus: true,
    closeArmed: false,
    closeLabel: "Close tab",
    closeTabIndex: -1,
    closeTitle: "Close tab",
    index: 0,
    itemKey: "tab",
    label: "tab",
    metaLabel: "tab",
    tabId: "tab",
    tabIndex: -1,
    title: "tab",
    ...values,
    ...overrides,
  };
}
