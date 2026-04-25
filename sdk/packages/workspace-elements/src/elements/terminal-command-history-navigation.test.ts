import { describe, expect, it } from "vitest";

import {
  canNavigateTerminalCommandHistory,
  createTerminalCommandHistoryNavigationState,
  resolveTerminalCommandHistoryNavigation,
  type TerminalCommandHistoryInputState,
} from "./terminal-command-history-navigation.js";

describe("terminal command history navigation", () => {
  it("walks backward through history and restores the pre-navigation draft", () => {
    const history = ["pwd", "ls -la", "git status"];
    const input = createInput("draft command");
    let state = createTerminalCommandHistoryNavigationState();

    const previousLatest = resolveTerminalCommandHistoryNavigation("previous", input, history, state);

    expect(previousLatest).toMatchObject({
      navigated: true,
      value: "git status",
      state: {
        cursor: 2,
        draftBeforeNavigation: "draft command",
      },
    });

    state = previousLatest.state;
    const previousOlder = resolveTerminalCommandHistoryNavigation("previous", createInput("git status"), history, state);

    expect(previousOlder).toMatchObject({
      navigated: true,
      value: "ls -la",
      state: {
        cursor: 1,
        draftBeforeNavigation: "draft command",
      },
    });

    state = previousOlder.state;
    const nextNewer = resolveTerminalCommandHistoryNavigation("next", createInput("ls -la"), history, state);

    expect(nextNewer).toMatchObject({
      navigated: true,
      value: "git status",
      state: {
        cursor: 2,
        draftBeforeNavigation: "draft command",
      },
    });

    state = nextNewer.state;
    const restoredDraft = resolveTerminalCommandHistoryNavigation("next", createInput("git status"), history, state);

    expect(restoredDraft).toEqual({
      navigated: true,
      value: "draft command",
      state: createTerminalCommandHistoryNavigationState(),
    });
  });

  it("keeps state unchanged when there is no history or no active cursor", () => {
    const state = createTerminalCommandHistoryNavigationState();

    expect(resolveTerminalCommandHistoryNavigation("previous", createInput("draft"), [], state)).toEqual({
      navigated: false,
      state,
    });
    expect(resolveTerminalCommandHistoryNavigation("next", createInput("draft"), ["pwd"], state)).toEqual({
      navigated: false,
      state,
    });
  });

  it("only navigates when the caret is on a command boundary", () => {
    expect(canNavigateTerminalCommandHistory("previous", createInput("one\ntwo", 3, 3))).toBe(true);
    expect(canNavigateTerminalCommandHistory("previous", createInput("one\ntwo", 5, 5))).toBe(false);
    expect(canNavigateTerminalCommandHistory("next", createInput("one\ntwo", 4, 4))).toBe(true);
    expect(canNavigateTerminalCommandHistory("next", createInput("one\ntwo", 2, 2))).toBe(false);
    expect(canNavigateTerminalCommandHistory("previous", createInput("draft", 1, 4))).toBe(false);
  });
});

function createInput(value: string, selectionStart = value.length, selectionEnd = selectionStart): TerminalCommandHistoryInputState {
  return {
    value,
    selectionStart,
    selectionEnd,
  };
}
