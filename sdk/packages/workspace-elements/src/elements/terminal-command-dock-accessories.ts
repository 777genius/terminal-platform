export const TERMINAL_COMMAND_DOCK_ACCESSORY_MODES = {
  bar: "bar",
  stack: "stack",
} as const;

export type TerminalCommandDockAccessoryMode =
  (typeof TERMINAL_COMMAND_DOCK_ACCESSORY_MODES)[keyof typeof TERMINAL_COMMAND_DOCK_ACCESSORY_MODES];

export type TerminalCommandDockAccessoryOptions = {
  placement?: string | null;
};

export interface TerminalCommandDockAccessoryStateOptions extends TerminalCommandDockAccessoryOptions {
  quickCommandCount?: number | null;
  recentCommandCount?: number | null;
}

export interface TerminalCommandDockAccessoryState {
  readonly mode: TerminalCommandDockAccessoryMode;
  readonly hasQuickCommands: boolean;
  readonly hasRecentCommands: boolean;
  readonly quickCommandCount: number;
  readonly recentCommandCount: number;
}

export function resolveTerminalCommandDockAccessoryMode(
  options: TerminalCommandDockAccessoryOptions = {},
): TerminalCommandDockAccessoryMode {
  return resolveTerminalCommandDockAccessoryState(options).mode;
}

export function resolveTerminalCommandDockAccessoryState(
  options: TerminalCommandDockAccessoryStateOptions = {},
): TerminalCommandDockAccessoryState {
  const quickCommandCount = normalizeAccessoryCount(options.quickCommandCount);
  const recentCommandCount = normalizeAccessoryCount(options.recentCommandCount);

  return {
    mode: options.placement === "terminal"
      ? TERMINAL_COMMAND_DOCK_ACCESSORY_MODES.bar
      : TERMINAL_COMMAND_DOCK_ACCESSORY_MODES.stack,
    hasQuickCommands: quickCommandCount > 0,
    hasRecentCommands: recentCommandCount > 0,
    quickCommandCount,
    recentCommandCount,
  };
}

function normalizeAccessoryCount(value: number | null | undefined): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return 0;
  }

  return Math.max(0, Math.trunc(value));
}
