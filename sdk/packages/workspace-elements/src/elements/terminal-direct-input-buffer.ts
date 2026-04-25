export const TERMINAL_DIRECT_INPUT_FLUSH_DELAY_MS = 16;

export interface TerminalDirectInputBufferOptions {
  readonly flush: (input: string) => void;
  readonly flushDelayMs?: number;
  readonly schedule?: (callback: () => void, delayMs: number) => ReturnType<typeof setTimeout>;
  readonly cancel?: (timer: ReturnType<typeof setTimeout>) => void;
}

export class TerminalDirectInputBuffer {
  readonly #flush: (input: string) => void;
  readonly #flushDelayMs: number;
  readonly #schedule: (callback: () => void, delayMs: number) => ReturnType<typeof setTimeout>;
  readonly #cancel: (timer: ReturnType<typeof setTimeout>) => void;
  #pendingInput = "";
  #flushTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(options: TerminalDirectInputBufferOptions) {
    this.#flush = options.flush;
    this.#flushDelayMs = options.flushDelayMs ?? TERMINAL_DIRECT_INPUT_FLUSH_DELAY_MS;
    this.#schedule = options.schedule ?? ((callback, delayMs) => globalThis.setTimeout(callback, delayMs));
    this.#cancel = options.cancel ?? ((timer) => globalThis.clearTimeout(timer));
  }

  push(input: string): void {
    if (shouldBufferTerminalDirectInput(input)) {
      this.#pendingInput += input;
      this.#scheduleFlush();
      return;
    }

    this.flush();
    this.#flush(input);
  }

  flush(): void {
    this.#clearFlushTimer();
    if (!this.#pendingInput) {
      return;
    }

    const input = this.#pendingInput;
    this.#pendingInput = "";
    this.#flush(input);
  }

  dispose(): void {
    this.flush();
  }

  #scheduleFlush(): void {
    if (this.#flushTimer) {
      return;
    }

    this.#flushTimer = this.#schedule(() => {
      this.#flushTimer = null;
      this.flush();
    }, this.#flushDelayMs);
  }

  #clearFlushTimer(): void {
    if (!this.#flushTimer) {
      return;
    }

    this.#cancel(this.#flushTimer);
    this.#flushTimer = null;
  }
}

export function shouldBufferTerminalDirectInput(input: string): boolean {
  return input.length === 1
    && input >= " "
    && input !== "\u007f"
    && input !== "\u001b";
}

export function shouldRefreshAfterTerminalDirectInput(input: string): boolean {
  return input.includes("\r")
    || input.includes("\u0003")
    || input.includes("\u0004");
}
