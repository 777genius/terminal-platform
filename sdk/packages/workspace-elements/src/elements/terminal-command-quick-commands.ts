export const TERMINAL_COMMAND_QUICK_COMMAND_LIMIT = 8;
export const TERMINAL_COMMAND_QUICK_COMMAND_TONES = Object.freeze({
  primary: "primary",
  secondary: "secondary",
} as const);

export type TerminalCommandQuickCommandTone =
  (typeof TERMINAL_COMMAND_QUICK_COMMAND_TONES)[keyof typeof TERMINAL_COMMAND_QUICK_COMMAND_TONES];

export interface TerminalCommandQuickCommand {
  readonly id?: string;
  readonly label: string;
  readonly value: string;
  readonly description?: string;
  readonly ariaLabel?: string;
  readonly tone?: TerminalCommandQuickCommandTone;
}

export interface TerminalCommandQuickCommandPresentation {
  readonly id: string;
  readonly label: string;
  readonly value: string;
  readonly title: string;
  readonly ariaLabel: string;
  readonly tone: TerminalCommandQuickCommandTone;
  readonly description?: string;
}

export const defaultTerminalCommandQuickCommands = Object.freeze([
  {
    id: "pwd",
    label: "pwd",
    value: "pwd",
    description: "Insert pwd",
  },
  {
    id: "list-files",
    label: "ls -la",
    value: "ls -la",
    description: "Insert ls -la",
  },
  {
    id: "git-status",
    label: "git status",
    value: "git status",
    description: "Insert git status",
  },
  {
    id: "hello",
    label: "hello",
    value: 'printf "hello from Terminal Platform\\n"',
    description: "Insert Terminal Platform hello command",
  },
] satisfies TerminalCommandQuickCommand[]);

export function resolveTerminalCommandQuickCommands(
  quickCommands: readonly TerminalCommandQuickCommand[] | null | undefined,
): TerminalCommandQuickCommandPresentation[] {
  const source: readonly TerminalCommandQuickCommand[] = quickCommands ?? defaultTerminalCommandQuickCommands;
  const resolved: TerminalCommandQuickCommandPresentation[] = [];
  const usedIds = new Map<string, number>();

  for (const [index, quickCommand] of source.entries()) {
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

    const description = trimOptionalString(quickCommand.description);
    const explicitAriaLabel = trimOptionalString(quickCommand.ariaLabel);
    const fallbackLabel = description ?? `Insert ${label}`;
    resolved.push({
      id: resolveQuickCommandId(quickCommand.id, index, usedIds),
      label,
      value,
      title: description ?? explicitAriaLabel ?? fallbackLabel,
      ariaLabel: explicitAriaLabel ?? fallbackLabel,
      tone: resolveQuickCommandTone(quickCommand.tone),
      ...(description ? { description } : {}),
    });

    if (resolved.length >= TERMINAL_COMMAND_QUICK_COMMAND_LIMIT) {
      break;
    }
  }

  return resolved;
}

function resolveQuickCommandId(
  id: string | undefined,
  index: number,
  usedIds: Map<string, number>,
): string {
  const candidate = typeof id === "string"
    ? id
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9_.:-]+/g, "-")
      .replace(/^-+|-+$/g, "")
    : "";
  const baseId = candidate || `quick-${index + 1}`;
  const usedCount = usedIds.get(baseId) ?? 0;
  usedIds.set(baseId, usedCount + 1);
  return usedCount === 0 ? baseId : `${baseId}-${usedCount + 1}`;
}

function resolveQuickCommandTone(
  tone: TerminalCommandQuickCommandTone | undefined,
): TerminalCommandQuickCommandTone {
  return tone === TERMINAL_COMMAND_QUICK_COMMAND_TONES.primary
    ? TERMINAL_COMMAND_QUICK_COMMAND_TONES.primary
    : TERMINAL_COMMAND_QUICK_COMMAND_TONES.secondary;
}

function trimOptionalString(value: string | undefined): string | undefined {
  return typeof value === "string" ? value.trim() || undefined : undefined;
}

function summarizeQuickCommandValue(value: string): string {
  return value.trim().replace(/\s+/g, " ").slice(0, 32);
}
