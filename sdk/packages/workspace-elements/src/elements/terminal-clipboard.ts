export const TERMINAL_CLIPBOARD_OPERATION_TIMEOUT_MS = 2500;

export interface TerminalClipboardAdapter {
  readonly readText?: () => Promise<string>;
  readonly writeText?: (value: string) => Promise<void>;
}

export interface TerminalClipboardOperationOptions {
  readonly clipboard?: TerminalClipboardAdapter | null;
  readonly timeoutMs?: number;
}

export async function readClipboardText(options: TerminalClipboardOperationOptions = {}): Promise<string> {
  const clipboard = resolveClipboardAdapter(options);
  if (!clipboard?.readText) {
    throw new Error("Clipboard read is unavailable in this browser context");
  }

  return withClipboardTimeout(
    clipboard.readText(),
    options.timeoutMs,
    "Clipboard read timed out. Check browser clipboard permissions and try again.",
  );
}

export async function writeClipboardText(
  value: string,
  options: TerminalClipboardOperationOptions = {},
): Promise<void> {
  const clipboard = resolveClipboardAdapter(options);
  if (!clipboard?.writeText) {
    throw new Error("Clipboard write is unavailable in this browser context");
  }

  await withClipboardTimeout(
    clipboard.writeText(value),
    options.timeoutMs,
    "Clipboard write timed out. Check browser clipboard permissions and try again.",
  );
}

function resolveClipboardAdapter(options: TerminalClipboardOperationOptions): TerminalClipboardAdapter | null | undefined {
  return Object.hasOwn(options, "clipboard")
    ? options.clipboard
    : globalThis.navigator?.clipboard;
}

function withClipboardTimeout<T>(
  operation: Promise<T>,
  timeoutMs = TERMINAL_CLIPBOARD_OPERATION_TIMEOUT_MS,
  timeoutMessage: string,
): Promise<T> {
  let timeoutId: ReturnType<typeof setTimeout> | null = null;

  const timeout = new Promise<never>((_, reject) => {
    timeoutId = globalThis.setTimeout(() => {
      reject(new Error(timeoutMessage));
    }, timeoutMs);
  });

  return Promise.race([operation, timeout]).finally(() => {
    if (timeoutId) {
      globalThis.clearTimeout(timeoutId);
    }
  });
}
