import { normalizeCommandHistoryEntry } from "../read-models/workspace-snapshot.js";

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

  merge(values: readonly string[]): void {
    const normalized = values
      .map((value) => normalizeCommandHistoryEntry(value))
      .filter((entry): entry is string => Boolean(entry));

    if (normalized.length === 0) {
      return;
    }

    this.#context.updateSnapshot((snapshot) => {
      const entries = [...snapshot.commandHistory.entries];
      for (const entry of normalized) {
        const existingIndex = entries.indexOf(entry);
        if (existingIndex >= 0) {
          entries.splice(existingIndex, 1);
        }
        entries.push(entry);
      }

      return {
        ...snapshot,
        commandHistory: {
          ...snapshot.commandHistory,
          entries: entries.slice(-snapshot.commandHistory.limit),
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
