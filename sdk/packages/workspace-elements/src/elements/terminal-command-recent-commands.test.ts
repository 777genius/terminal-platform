import { describe, expect, it } from "vitest";

import { resolveTerminalCommandRecentCommands } from "./terminal-command-recent-commands.js";

describe("terminal command recent commands", () => {
  it("resolves recent commands into stable presentation entries", () => {
    expect(resolveTerminalCommandRecentCommands(["pwd", "ls -la", "git status", "npm test"], 2)).toEqual([
      {
        id: "history-4",
        index: 0,
        historyIndex: 3,
        label: "npm test",
        value: "npm test",
        title: "npm test",
        ariaLabel: "Use recent command npm test",
      },
      {
        id: "history-3",
        index: 1,
        historyIndex: 2,
        label: "git status",
        value: "git status",
        title: "git status",
        ariaLabel: "Use recent command git status",
      },
    ]);
  });

  it("preserves duplicate command values by using their history position", () => {
    const entries = resolveTerminalCommandRecentCommands(["pwd", "pwd", "pwd"], 3);

    expect(entries.map((entry) => entry.id)).toEqual(["history-3", "history-2", "history-1"]);
    expect(entries.map((entry) => entry.value)).toEqual(["pwd", "pwd", "pwd"]);
  });

  it("returns no presentation entries when the host hides recent commands", () => {
    expect(resolveTerminalCommandRecentCommands(["pwd", "ls -la"], 0)).toEqual([]);
  });
});
