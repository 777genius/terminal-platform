export const TERMINAL_COMMAND_DOCK_ACCESSORY_MODES = {
  bar: "bar",
  stack: "stack",
} as const;

export type TerminalCommandDockAccessoryMode =
  (typeof TERMINAL_COMMAND_DOCK_ACCESSORY_MODES)[keyof typeof TERMINAL_COMMAND_DOCK_ACCESSORY_MODES];

export type TerminalCommandDockAccessoryOptions = {
  placement?: string | null;
};

export function resolveTerminalCommandDockAccessoryMode(
  options: TerminalCommandDockAccessoryOptions = {},
): TerminalCommandDockAccessoryMode {
  return options.placement === "terminal"
    ? TERMINAL_COMMAND_DOCK_ACCESSORY_MODES.bar
    : TERMINAL_COMMAND_DOCK_ACCESSORY_MODES.stack;
}
