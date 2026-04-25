export type TerminalCommandHistoryNavigationDirection = "previous" | "next";

export type TerminalCommandHistoryNavigationState = {
  cursor: number | null;
  draftBeforeNavigation: string;
};

export type TerminalCommandHistoryInputState = {
  value: string;
  selectionStart: number;
  selectionEnd: number;
};

export type TerminalCommandHistoryNavigationResult =
  | {
    navigated: false;
    state: TerminalCommandHistoryNavigationState;
  }
  | {
    navigated: true;
    state: TerminalCommandHistoryNavigationState;
    value: string;
  };

export function createTerminalCommandHistoryNavigationState(): TerminalCommandHistoryNavigationState {
  return {
    cursor: null,
    draftBeforeNavigation: "",
  };
}

export function canNavigateTerminalCommandHistory(
  direction: TerminalCommandHistoryNavigationDirection,
  input: TerminalCommandHistoryInputState,
): boolean {
  if (input.selectionStart !== input.selectionEnd) {
    return false;
  }

  if (direction === "previous") {
    return !input.value.slice(0, input.selectionStart).includes("\n");
  }

  return !input.value.slice(input.selectionEnd).includes("\n");
}

export function resolveTerminalCommandHistoryNavigation(
  direction: TerminalCommandHistoryNavigationDirection,
  input: TerminalCommandHistoryInputState,
  commandHistory: readonly string[],
  state: TerminalCommandHistoryNavigationState,
): TerminalCommandHistoryNavigationResult {
  if (commandHistory.length === 0 || !canNavigateTerminalCommandHistory(direction, input)) {
    return { navigated: false, state };
  }

  if (direction === "previous") {
    const nextState = {
      cursor: state.cursor === null
        ? commandHistory.length - 1
        : Math.max(0, state.cursor - 1),
      draftBeforeNavigation: state.cursor === null ? input.value : state.draftBeforeNavigation,
    };
    const historyDraft = commandHistory[nextState.cursor];

    return historyDraft
      ? { navigated: true, state: nextState, value: historyDraft }
      : { navigated: false, state };
  }

  if (state.cursor === null) {
    return { navigated: false, state };
  }

  if (state.cursor === commandHistory.length - 1) {
    return {
      navigated: true,
      state: createTerminalCommandHistoryNavigationState(),
      value: state.draftBeforeNavigation,
    };
  }

  const nextState = {
    cursor: state.cursor + 1,
    draftBeforeNavigation: state.draftBeforeNavigation,
  };
  const historyDraft = commandHistory[nextState.cursor];

  return historyDraft
    ? { navigated: true, state: nextState, value: historyDraft }
    : { navigated: false, state };
}
