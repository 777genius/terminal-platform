export const TERMINAL_COMMAND_COMPOSER_DEFAULT_MIN_ROWS = 1;
export const TERMINAL_COMMAND_COMPOSER_DEFAULT_MAX_ROWS = 5;

export type TerminalCommandComposerRowOptions = {
  minRows?: number | null;
  maxRows?: number | null;
};

export type TerminalCommandComposerRowRange = {
  minRows: number;
  maxRows: number;
};

export function resolveTerminalCommandComposerRowRange(
  options: TerminalCommandComposerRowOptions = {},
): TerminalCommandComposerRowRange {
  const minRows = normalizeTerminalCommandComposerRows(
    options.minRows,
    TERMINAL_COMMAND_COMPOSER_DEFAULT_MIN_ROWS,
  );
  const requestedMaxRows = normalizeTerminalCommandComposerRows(
    options.maxRows,
    TERMINAL_COMMAND_COMPOSER_DEFAULT_MAX_ROWS,
  );

  return {
    minRows,
    maxRows: Math.max(minRows, requestedMaxRows),
  };
}

export function resolveTerminalCommandComposerRows(
  value: string,
  options: TerminalCommandComposerRowOptions = {},
): number {
  const { minRows, maxRows } = resolveTerminalCommandComposerRowRange(options);
  const lineCount = countLogicalLines(value);
  return Math.min(maxRows, Math.max(minRows, lineCount));
}

function normalizeTerminalCommandComposerRows(value: number | null | undefined, fallback: number): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return fallback;
  }

  return Math.max(1, Math.trunc(value));
}

function countLogicalLines(value: string): number {
  if (value.length === 0) {
    return 1;
  }

  return value.split(/\r\n|\r|\n/).length;
}
