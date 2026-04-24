import type { ServiceContext } from "./service-context.js";

export class CommandHistoryService {
  readonly #context: Pick<ServiceContext, "updateSnapshot">;

  constructor(context: Pick<ServiceContext, "updateSnapshot">) {
    this.#context = context;
  }

  record(value: string): void {
    const entry = normalizeCommandHistoryEntry(value);
    if (!entry) {
      return;
    }

    this.#context.updateSnapshot((snapshot) => {
      const entries = snapshot.commandHistory.entries.filter((current) => current !== entry);
      return {
        ...snapshot,
        commandHistory: {
          ...snapshot.commandHistory,
          entries: [...entries, entry].slice(-snapshot.commandHistory.limit),
        },
      };
    });
  }

  clear(): void {
    this.#context.updateSnapshot((snapshot) => ({
      ...snapshot,
      commandHistory: {
        ...snapshot.commandHistory,
        entries: [],
      },
    }));
  }
}

function normalizeCommandHistoryEntry(value: string): string | null {
  const entry = value.replace(/\s+$/u, "");
  return entry.trim().length > 0 ? entry : null;
}
