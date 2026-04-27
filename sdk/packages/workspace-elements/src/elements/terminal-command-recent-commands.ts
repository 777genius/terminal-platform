export interface TerminalCommandRecentCommandPresentation {
  readonly id: string;
  readonly index: number;
  readonly historyIndex: number;
  readonly label: string;
  readonly value: string;
  readonly title: string;
  readonly ariaLabel: string;
}

export function resolveTerminalCommandRecentCommands(
  commandHistory: readonly string[],
  limit: number,
): TerminalCommandRecentCommandPresentation[] {
  if (limit <= 0 || commandHistory.length === 0) {
    return [];
  }

  const startIndex = Math.max(0, commandHistory.length - limit);
  return commandHistory
    .slice(startIndex)
    .map((command, offset) => ({
      command,
      historyIndex: startIndex + offset,
    }))
    .reverse()
    .map(({ command, historyIndex }, index) => ({
      id: `history-${historyIndex + 1}`,
      index,
      historyIndex,
      label: command,
      value: command,
      title: command,
      ariaLabel: `Use recent command ${command}`,
    }));
}
