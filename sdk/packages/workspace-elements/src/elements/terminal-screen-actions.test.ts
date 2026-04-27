import { describe, expect, it } from "vitest";

import {
  TERMINAL_SCREEN_ACTION_IDS,
  resolveTerminalScreenActions,
} from "./terminal-screen-actions.js";

describe("terminal screen actions", () => {
  it("uses compact labels for terminal placement", () => {
    const actions = resolveTerminalScreenActions({
      canCopyVisibleOutput: true,
      followOutput: true,
      placement: "terminal",
    });

    expect(actions.map((action) => action.id)).toEqual([
      TERMINAL_SCREEN_ACTION_IDS.followOutput,
      TERMINAL_SCREEN_ACTION_IDS.scrollLatest,
      TERMINAL_SCREEN_ACTION_IDS.copyVisible,
    ]);
    expect(actions.map((action) => action.label)).toEqual(["\u23f8", "\u2193", "\u2398"]);
    expect(actions.map((action) => action.labelMode)).toEqual(["glyph", "glyph", "glyph"]);
    expect(actions.map((action) => action.placement)).toEqual(["terminal", "terminal", "terminal"]);
    expect(actions.map((action) => action.testId)).toEqual([
      "tp-screen-follow",
      "tp-screen-scroll-latest",
      "tp-screen-copy",
    ]);
    expect(actions.map((action) => action.tone)).toEqual(["primary", "secondary", "secondary"]);
    expect(actions[0]).toMatchObject({
      ariaLabel: "Pause automatic terminal output follow",
      ariaPressed: true,
      title: "Pause automatic terminal output follow",
    });
    expect(actions[1]).toMatchObject({
      ariaLabel: "Scroll to latest terminal output",
      title: "Scroll to latest terminal output",
    });
  });

  it("keeps descriptive labels for panel placement", () => {
    const actions = resolveTerminalScreenActions({
      canCopyVisibleOutput: true,
      followOutput: true,
      placement: "panel",
    });

    expect(actions.map((action) => action.label)).toEqual(["Following", "Scroll latest", "Copy visible"]);
    expect(actions.map((action) => action.labelMode)).toEqual(["label", "label", "label"]);
    expect(actions.map((action) => action.placement)).toEqual(["panel", "panel", "panel"]);
  });

  it("models paused follow state without disabling manual scroll", () => {
    const actions = resolveTerminalScreenActions({
      canCopyVisibleOutput: true,
      followOutput: false,
      placement: "terminal",
    });

    expect(actions[0]).toMatchObject({
      ariaLabel: "Follow terminal output",
      ariaPressed: false,
      disabled: false,
      label: "\u25b6",
      labelMode: "glyph",
      title: "Follow terminal output",
      tone: "secondary",
    });
    expect(actions[1]).toMatchObject({ disabled: false, label: "\u2193" });
  });

  it("keeps visible-output copy state explicit", () => {
    expect(resolveTerminalScreenActions({
      canCopyVisibleOutput: true,
      copyState: "copied",
      placement: "terminal",
    })[2]).toMatchObject({
      ariaLabel: "Visible terminal output copied",
      disabled: false,
      label: "\u2713",
      labelMode: "glyph",
      title: "Visible terminal output copied",
      tone: "success",
    });

    expect(resolveTerminalScreenActions({
      canCopyVisibleOutput: true,
      copyState: "failed",
      placement: "terminal",
    })[2]).toMatchObject({
      ariaLabel: "Copy visible terminal output failed",
      disabled: false,
      label: "!",
      labelMode: "glyph",
      title: "Visible terminal output could not be copied",
      tone: "danger",
    });

    expect(resolveTerminalScreenActions({
      canCopyVisibleOutput: true,
      copyState: "failed",
      placement: "panel",
    })[2]?.label).toBe("Copy failed");
  });

  it("disables copy while no visible output is available", () => {
    const copyAction = resolveTerminalScreenActions({
      canCopyVisibleOutput: false,
      placement: "terminal",
    })[2];

    expect(copyAction).toMatchObject({
      ariaLabel: "Copy visible terminal output",
      disabled: true,
      label: "\u2398",
      title: "Copy visible terminal output",
    });
  });
});
