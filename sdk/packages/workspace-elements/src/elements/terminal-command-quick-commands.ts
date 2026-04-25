export const TERMINAL_COMMAND_QUICK_COMMAND_LIMIT = 8;

export interface TerminalCommandQuickCommand {
  readonly label: string;
  readonly value: string;
  readonly description?: string;
}

export const defaultTerminalCommandQuickCommands = Object.freeze([
  {
    label: "pwd",
    value: "pwd",
    description: "Insert pwd",
  },
  {
    label: "ls -la",
    value: "ls -la",
    description: "Insert ls -la",
  },
  {
    label: "git status",
    value: "git status",
    description: "Insert git status",
  },
  {
    label: "hello",
    value: 'printf "hello from Terminal Platform\\n"',
    description: "Insert Terminal Platform hello command",
  },
] satisfies TerminalCommandQuickCommand[]);

export function resolveTerminalCommandQuickCommands(
  quickCommands: readonly TerminalCommandQuickCommand[] | null | undefined,
): TerminalCommandQuickCommand[] {
  const source = quickCommands ?? defaultTerminalCommandQuickCommands;
  const resolved: TerminalCommandQuickCommand[] = [];

  for (const quickCommand of source) {
    if (
      !quickCommand
      || typeof quickCommand.label !== "string"
      || typeof quickCommand.value !== "string"
    ) {
      continue;
    }

    const value = quickCommand.value;
    if (value.trim().length === 0) {
      continue;
    }

    const label = quickCommand.label.trim() || summarizeQuickCommandValue(value);
    if (!label) {
      continue;
    }

    const description = quickCommand.description?.trim();
    resolved.push({
      label,
      value,
      ...(description ? { description } : {}),
    });

    if (resolved.length >= TERMINAL_COMMAND_QUICK_COMMAND_LIMIT) {
      break;
    }
  }

  return resolved;
}

function summarizeQuickCommandValue(value: string): string {
  return value.trim().replace(/\s+/g, " ").slice(0, 32);
}
