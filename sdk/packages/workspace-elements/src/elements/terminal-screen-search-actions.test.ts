import { describe, expect, it } from "vitest";

import {
  TERMINAL_SCREEN_SEARCH_ACTION_IDS,
  resolveTerminalScreenSearchActions,
} from "./terminal-screen-search-actions.js";

describe("terminal screen search actions", () => {
  it("uses compact glyphs for terminal placement", () => {
    const actions = resolveTerminalScreenSearchActions({
      matchCount: 2,
      placement: "terminal",
      query: "browser-smoke-ok",
    });

    expect(actions.map((action) => action.id)).toEqual([
      TERMINAL_SCREEN_SEARCH_ACTION_IDS.previousMatch,
      TERMINAL_SCREEN_SEARCH_ACTION_IDS.nextMatch,
      TERMINAL_SCREEN_SEARCH_ACTION_IDS.clearSearch,
    ]);
    expect(actions.map((action) => action.label)).toEqual(["\u2191", "\u2193", "\u00d7"]);
    expect(actions.map((action) => action.labelMode)).toEqual(["glyph", "glyph", "glyph"]);
    expect(actions.map((action) => action.placement)).toEqual(["terminal", "terminal", "terminal"]);
    expect(actions.map((action) => action.testId)).toEqual([
      "tp-screen-search-prev",
      "tp-screen-search-next",
      "tp-screen-search-clear",
    ]);
    expect(actions.map((action) => action.disabled)).toEqual([false, false, false]);
    expect(actions[0]).toMatchObject({
      ariaLabel: "Select previous search match",
      title: "Select previous search match",
      tone: "secondary",
    });
  });

  it("keeps descriptive labels for panel placement", () => {
    const actions = resolveTerminalScreenSearchActions({
      matchCount: 1,
      placement: "panel",
      query: "ok",
    });

    expect(actions.map((action) => action.label)).toEqual(["Prev", "Next", "Clear"]);
    expect(actions.map((action) => action.labelMode)).toEqual(["label", "label", "label"]);
    expect(actions.map((action) => action.placement)).toEqual(["panel", "panel", "panel"]);
  });

  it("disables match navigation without matches but keeps clear available for a query", () => {
    const actions = resolveTerminalScreenSearchActions({
      matchCount: 0,
      placement: "terminal",
      query: "missing",
    });

    expect(actions.map((action) => action.disabled)).toEqual([true, true, false]);
  });

  it("normalizes missing state for an idle search", () => {
    const actions = resolveTerminalScreenSearchActions({
      matchCount: Number.NaN,
      placement: "unknown",
      query: "",
    });

    expect(actions.map((action) => action.disabled)).toEqual([true, true, true]);
    expect(actions.map((action) => action.labelMode)).toEqual(["label", "label", "label"]);
    expect(actions.map((action) => action.placement)).toEqual(["panel", "panel", "panel"]);
  });
});
