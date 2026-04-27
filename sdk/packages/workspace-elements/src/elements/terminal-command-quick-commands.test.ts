import { describe, expect, it } from "vitest";

import {
  TERMINAL_COMMAND_QUICK_COMMAND_LIMIT,
  TERMINAL_COMMAND_QUICK_COMMAND_TONES,
  defaultTerminalCommandQuickCommands,
  resolveTerminalCommandQuickCommands,
  type TerminalCommandQuickCommand,
} from "./terminal-command-quick-commands.js";

describe("terminal command quick commands", () => {
  it("returns the default production quick commands when no override is provided", () => {
    expect(resolveTerminalCommandQuickCommands(undefined).map(({ id, label, tone }) => ({ id, label, tone }))).toEqual([
      { id: "pwd", label: "pwd", tone: TERMINAL_COMMAND_QUICK_COMMAND_TONES.secondary },
      { id: "list-files", label: "ls -la", tone: TERMINAL_COMMAND_QUICK_COMMAND_TONES.secondary },
      { id: "git-status", label: "git status", tone: TERMINAL_COMMAND_QUICK_COMMAND_TONES.secondary },
      { id: "hello", label: "hello", tone: TERMINAL_COMMAND_QUICK_COMMAND_TONES.secondary },
    ]);
    expect(defaultTerminalCommandQuickCommands[0]?.id).toBe("pwd");
  });

  it("preserves host supplied command values while resolving presentation metadata", () => {
    expect(resolveTerminalCommandQuickCommands([
      {
        id: "  Smoke Command  ",
        label: "  smoke  ",
        value: 'printf "ok\\n"',
        description: "  Insert smoke command  ",
        tone: TERMINAL_COMMAND_QUICK_COMMAND_TONES.primary,
      },
    ])).toEqual([
      {
        id: "smoke-command",
        label: "smoke",
        value: 'printf "ok\\n"',
        title: "Insert smoke command",
        ariaLabel: "Insert smoke command",
        tone: TERMINAL_COMMAND_QUICK_COMMAND_TONES.primary,
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
        id: "quick-2",
        label: 'printf "derived label\\n"',
        value: "printf   \"derived label\\n\"",
        title: 'Insert printf "derived label\\n"',
        ariaLabel: 'Insert printf "derived label\\n"',
        tone: TERMINAL_COMMAND_QUICK_COMMAND_TONES.secondary,
      },
    ]);
  });

  it("keeps duplicate and malformed ids stable without using command values", () => {
    expect(resolveTerminalCommandQuickCommands([
      { id: "Git Status", label: "git", value: "git status" },
      { id: "git-status", label: "git short", value: "git status --short" },
      { id: "   ", label: "fallback", value: "echo fallback", ariaLabel: "  Insert fallback  " },
    ])).toEqual([
      {
        id: "git-status",
        label: "git",
        value: "git status",
        title: "Insert git",
        ariaLabel: "Insert git",
        tone: TERMINAL_COMMAND_QUICK_COMMAND_TONES.secondary,
      },
      {
        id: "git-status-2",
        label: "git short",
        value: "git status --short",
        title: "Insert git short",
        ariaLabel: "Insert git short",
        tone: TERMINAL_COMMAND_QUICK_COMMAND_TONES.secondary,
      },
      {
        id: "quick-3",
        label: "fallback",
        value: "echo fallback",
        title: "Insert fallback",
        ariaLabel: "Insert fallback",
        tone: TERMINAL_COMMAND_QUICK_COMMAND_TONES.secondary,
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
        id: "quick-4",
        label: "good",
        value: "echo good",
        title: "Insert good",
        ariaLabel: "Insert good",
        tone: TERMINAL_COMMAND_QUICK_COMMAND_TONES.secondary,
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
