import { describe, expect, it } from "vitest";

import {
  TERMINAL_COMMAND_QUICK_COMMAND_LIMIT,
  defaultTerminalCommandQuickCommands,
  resolveTerminalCommandQuickCommands,
  type TerminalCommandQuickCommand,
} from "./terminal-command-quick-commands.js";

describe("terminal command quick commands", () => {
  it("returns the default production quick commands when no override is provided", () => {
    expect(resolveTerminalCommandQuickCommands(undefined)).toEqual(defaultTerminalCommandQuickCommands);
  });

  it("preserves host supplied command values while trimming labels and descriptions", () => {
    expect(resolveTerminalCommandQuickCommands([
      {
        label: "  smoke  ",
        value: 'printf "ok\\n"',
        description: "  Insert smoke command  ",
      },
    ])).toEqual([
      {
        label: "smoke",
        value: 'printf "ok\\n"',
        description: "Insert smoke command",
      },
    ]);
  });

  it("allows the host to hide quick commands with an empty list", () => {
    expect(resolveTerminalCommandQuickCommands([])).toEqual([]);
  });

  it("skips blank values and derives a compact label when needed", () => {
    expect(resolveTerminalCommandQuickCommands([
      { label: "blank", value: "   " },
      { label: "  ", value: "printf   \"derived label\\n\"" },
    ])).toEqual([
      {
        label: 'printf "derived label\\n"',
        value: "printf   \"derived label\\n\"",
      },
    ]);
  });

  it("ignores malformed runtime entries passed by plain JavaScript hosts", () => {
    const commands = [
      null,
      { label: "bad" },
      { label: 42, value: "echo bad" },
      { label: "good", value: "echo good" },
    ] as unknown as TerminalCommandQuickCommand[];

    expect(resolveTerminalCommandQuickCommands(commands)).toEqual([
      {
        label: "good",
        value: "echo good",
      },
    ]);
  });

  it("caps rendered actions to keep the command lane stable", () => {
    const commands = Array.from({ length: TERMINAL_COMMAND_QUICK_COMMAND_LIMIT + 3 }, (_, index) => ({
      label: `cmd-${index}`,
      value: `echo ${index}`,
    })) satisfies TerminalCommandQuickCommand[];

    const resolved = resolveTerminalCommandQuickCommands(commands);

    expect(resolved).toHaveLength(TERMINAL_COMMAND_QUICK_COMMAND_LIMIT);
    expect(resolved.at(-1)?.label).toBe(`cmd-${TERMINAL_COMMAND_QUICK_COMMAND_LIMIT - 1}`);
  });
});
