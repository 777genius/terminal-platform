import { describe, expect, it } from "vitest";

import type { WorkspaceSnapshot } from "@terminal-platform/workspace-core";

import {
  createTerminalHistoryEntries,
  createVisibleOutputLines,
  resolveScrollTopAfterHistoryPrepend,
  shouldAutoLoadMoreHistoryFromViewport,
} from "./terminal-screen-element.js";

type FocusedScreen = NonNullable<
  WorkspaceSnapshot["attachedSession"]
>["focused_screen"];
type HistoricalPane = NonNullable<WorkspaceSnapshot["historicalPanes"]>[string];

describe("terminal screen visible output", () => {
  it("marks partial restored history before live output", () => {
    const lines = createVisibleOutputLines(
      createHistory({ hasMoreSegments: true }),
      createScreen(["live prompt"])
    );

    expect(lines).toEqual([
      { text: "old command", source: "history" },
      { text: "old output", source: "history" },
      {
        text: "--- restored history is partial; more persisted output is available ---",
        source: "boundary",
      },
      {
        text: "--- restored history above; live process below ---",
        source: "boundary",
      },
      { text: "live prompt", source: "live" },
    ]);
  });

  it("marks partial restored history even when no live output is attached", () => {
    const lines = createVisibleOutputLines(
      createHistory({ hasMoreSegments: true }),
      createScreen([])
    );

    expect(lines.at(-1)).toEqual({
      text: "--- restored history is partial; more persisted output is available ---",
      source: "boundary",
    });
  });

  it("marks partial restored history when the first loaded page has no visible lines", () => {
    const lines = createVisibleOutputLines(
      createHistory({ hasMoreSegments: true, lines: [] }),
      createScreen(["live prompt"])
    );

    expect(lines).toEqual([
      {
        text: "--- restored history is partial; more persisted output is available ---",
        source: "boundary",
      },
      {
        text: "--- restored history above; live process below ---",
        source: "boundary",
      },
      { text: "live prompt", source: "live" },
    ]);
  });

  it("trims trailing blank live rows without dropping interior spacing", () => {
    const lines = createVisibleOutputLines(
      createHistory({ hasMoreSegments: false }),
      createScreen(["live prompt", "", "live output", "   ", ""])
    );

    expect(lines).toEqual([
      { text: "old command", source: "history" },
      { text: "old output", source: "history" },
      {
        text: "--- restored history above; live process below ---",
        source: "boundary",
      },
      { text: "live prompt", source: "live" },
      { text: "", source: "live" },
      { text: "live output", source: "live" },
    ]);
  });

  it("deduplicates restored history suffix that is already present in live output", () => {
    const lines = createVisibleOutputLines(
      createHistory({
        hasMoreSegments: false,
        lines: ["one", "two", "three"],
      }),
      createScreen(["two", "three", "four"])
    );

    expect(lines).toEqual([
      { text: "one", source: "history" },
      {
        text: "--- restored history above; live process below ---",
        source: "boundary",
      },
      { text: "two", source: "live" },
      { text: "three", source: "live" },
      { text: "four", source: "live" },
    ]);
  });

  it("hides restored history boundary when live output fully covers restored history", () => {
    const lines = createVisibleOutputLines(
      createHistory({
        hasMoreSegments: false,
        lines: ["two", "three"],
      }),
      createScreen(["two", "three", "four"])
    );

    expect(lines).toEqual([
      { text: "two", source: "live" },
      { text: "three", source: "live" },
      { text: "four", source: "live" },
    ]);
  });

  it("deduplicates restored history after command echo normalization", () => {
    const lines = createVisibleOutputLines(
      createHistory({
        hasMoreSegments: false,
        lines: [
          "<                      echo TP_VERIFY_1",
          "TP_VERIFY_1",
        ],
      }),
      createScreen([
        "shell % echo TP_VERIFY_1",
        "TP_VERIFY_1",
      ]),
      { hideShellPromptNoise: true, preserveShellPromptCommands: true }
    );

    expect(lines).toEqual([
      { text: "shell % echo TP_VERIFY_1", source: "live" },
      { text: "TP_VERIFY_1", source: "live" },
    ]);
  });

  it("keeps shell prompt noise by default", () => {
    const lines = createVisibleOutputLines(
      createHistory({ hasMoreSegments: false }),
      createScreen(["(venv312) (base) belief@MacBook-Pro-belief claude_team %"])
    );

    expect(lines.at(-1)).toEqual({
      text: "(venv312) (base) belief@MacBook-Pro-belief claude_team %",
      source: "live",
    });
  });

  it("filters shell prompt noise when requested", () => {
    const lines = createVisibleOutputLines(
      createHistory({
        hasMoreSegments: false,
        lines: ["old output", "belief@MacBook-Pro-belief claude_team %", "%"],
      }),
      createScreen([
        "(venv312) (base) belief@MacBook-Pro-belief claude_team %",
        '(venv312) (base) belief@MacBook-Pro-belief claude_team % printf "ok\\n"',
        "ok",
      ]),
      { hideShellPromptNoise: true }
    );

    expect(lines).toEqual([
      { text: "old output", source: "history" },
      { text: "ok", source: "live" },
    ]);
  });

  it("hides shell prompt command echo lines when requested", () => {
    const lines = createVisibleOutputLines(
      createHistory({
        hasMoreSegments: false,
        lines: [
          "(venv312) (base) belief@MacBook-Pro-belief terminal-ui-smoke % pnpm test",
        ],
      }),
      createScreen([
        'belief@MacBook-Pro-belief terminal-ui-smoke % printf "ok\\n"',
        "ok",
      ]),
      { hideShellPromptNoise: true }
    );

    expect(lines).toEqual([
      { text: "ok", source: "live" },
    ]);
  });

  it("keeps shell prompt command echo lines for terminal history grouping", () => {
    const lines = createVisibleOutputLines(
      null,
      createScreen([
        "(venv312) belief@MacBook-Pro-belief ~/dev/quanta % print 123;",
        "123",
      ]),
      { hideShellPromptNoise: true, preserveShellPromptCommands: true }
    );

    expect(lines).toEqual([
      {
        text: "(venv312) belief@MacBook-Pro-belief ~/dev/quanta % print 123;",
        source: "live",
      },
      { text: "123", source: "live" },
    ]);
  });

  it("groups prompt command lines with their output for terminal rendering", () => {
    const entries = createTerminalHistoryEntries([
      {
        text: "venv312 ~/dev/quanta ((0.345s)) % print 123;",
        source: "history",
      },
      { text: "123", source: "history" },
      {
        text: "venv312 ~/dev/quanta ((0.316s)) % print 222;",
        source: "live",
      },
      { text: "222", source: "live" },
    ]);

    expect(entries).toEqual([
      {
        kind: "command",
        prompt: "venv312 ~/dev/quanta ((0.345s))",
        commandLine: {
          text: "venv312 ~/dev/quanta ((0.345s)) % print 123;",
          source: "history",
        },
        commandLineIndex: 0,
        command: "print 123;",
        output: [{ line: { text: "123", source: "history" }, lineIndex: 1 }],
      },
      {
        kind: "command",
        prompt: "venv312 ~/dev/quanta ((0.316s))",
        commandLine: {
          text: "venv312 ~/dev/quanta ((0.316s)) % print 222;",
          source: "live",
        },
        commandLineIndex: 2,
        command: "print 222;",
        output: [{ line: { text: "222", source: "live" }, lineIndex: 3 }],
      },
    ]);
  });

  it("groups wrapped input command lines with their output for terminal rendering", () => {
    const lines = createVisibleOutputLines(
      null,
      createScreen([
        "<                      echo TP_USER_HISTORY_1781353735418",
        "TP_USER_HISTORY_1781353735418",
      ]),
      { hideShellPromptNoise: true, preserveShellPromptCommands: true }
    );

    expect(createTerminalHistoryEntries(lines)).toEqual([
      {
        kind: "command",
        prompt: "shell",
        commandLine: {
          text: "shell % echo TP_USER_HISTORY_1781353735418",
          source: "live",
        },
        commandLineIndex: 0,
        command: "echo TP_USER_HISTORY_1781353735418",
        output: [
          {
            line: { text: "TP_USER_HISTORY_1781353735418", source: "live" },
            lineIndex: 1,
          },
        ],
      },
    ]);
  });

  it("uses the host prompt label for wrapped terminal input rows", () => {
    const lines = createVisibleOutputLines(
      null,
      createScreen([
        "<                      echo TP_PROMPT_LABEL",
        "TP_PROMPT_LABEL",
      ]),
      {
        hideShellPromptNoise: true,
        preserveShellPromptCommands: true,
        terminalPromptLabel: "~/dev/quanta",
      }
    );

    expect(lines).toEqual([
      { text: "~/dev/quanta % echo TP_PROMPT_LABEL", source: "live" },
      { text: "TP_PROMPT_LABEL", source: "live" },
    ]);
    expect(
      createTerminalHistoryEntries(lines, {
        terminalPromptLabel: "~/dev/quanta",
      })
    ).toEqual([
      {
        kind: "command",
        prompt: "~/dev/quanta",
        commandLine: {
          text: "~/dev/quanta % echo TP_PROMPT_LABEL",
          source: "live",
        },
        commandLineIndex: 0,
        command: "echo TP_PROMPT_LABEL",
        output: [
          {
            line: { text: "TP_PROMPT_LABEL", source: "live" },
            lineIndex: 1,
          },
        ],
      },
    ]);
  });

  it("does not attach live output to the last restored history command", () => {
    const entries = createTerminalHistoryEntries([
      {
        text: "shell % echo TP_GROUP_1781355541945",
        source: "history",
      },
      { text: "TP_GROUP_1781355541945", source: "history" },
      { text: "TP_FINAL_1781337254713", source: "live" },
    ]);

    expect(entries).toEqual([
      {
        kind: "command",
        prompt: "shell",
        commandLine: {
          text: "shell % echo TP_GROUP_1781355541945",
          source: "history",
        },
        commandLineIndex: 0,
        command: "echo TP_GROUP_1781355541945",
        output: [
          {
            line: { text: "TP_GROUP_1781355541945", source: "history" },
            lineIndex: 1,
          },
        ],
      },
      {
        kind: "line",
        line: { text: "TP_FINAL_1781337254713", source: "live" },
        lineIndex: 2,
      },
    ]);
  });

  it("prefers live command entries over identical restored history entries", () => {
    const entries = createTerminalHistoryEntries([
      { text: "shell % echo TP_VERIFY_1", source: "history" },
      { text: "TP_VERIFY_1", source: "history" },
      { text: "shell % echo TP_VERIFY_1", source: "live" },
      { text: "TP_VERIFY_1", source: "live" },
    ]);

    expect(entries).toEqual([
      {
        kind: "command",
        prompt: "shell",
        commandLine: {
          text: "shell % echo TP_VERIFY_1",
          source: "live",
        },
        commandLineIndex: 2,
        command: "echo TP_VERIFY_1",
        output: [
          {
            line: { text: "TP_VERIFY_1", source: "live" },
            lineIndex: 3,
          },
        ],
      },
    ]);
  });

  it("hides wrapped shell prompt fragments from command echo lines when requested", () => {
    const lines = createVisibleOutputLines(
      null,
      createScreen([
        '(venv312) (base) belief@MacBook-Pro-belief terminal-ui-smoke % printf "TP_SHEE',
        'printf "TP_SHEE',
        'ef terminal-ui-smoke % printf "TP_SHEET',
        '<                      printf "TP_SHEET_FIX_1781342995101\\n"',
        'dquote> "',
        "TP_SHEET",
      ]),
      { hideShellPromptNoise: true }
    );

    expect(lines).toEqual([{ text: "TP_SHEET", source: "live" }]);
  });

  it("auto-loads older history only near the top while idle", () => {
    expect(
      shouldAutoLoadMoreHistoryFromViewport(
        { scrollTop: 0 } as HTMLElement,
        true,
        "idle"
      )
    ).toBe(true);
    expect(
      shouldAutoLoadMoreHistoryFromViewport(
        { scrollTop: 24 } as HTMLElement,
        true,
        "idle"
      )
    ).toBe(true);
    expect(
      shouldAutoLoadMoreHistoryFromViewport(
        { scrollTop: 25 } as HTMLElement,
        true,
        "idle"
      )
    ).toBe(false);
    expect(
      shouldAutoLoadMoreHistoryFromViewport(
        { scrollTop: 0 } as HTMLElement,
        false,
        "idle"
      )
    ).toBe(false);
    expect(
      shouldAutoLoadMoreHistoryFromViewport(
        { scrollTop: 0 } as HTMLElement,
        true,
        "loading"
      )
    ).toBe(false);
  });

  it("preserves the viewport anchor after prepending older history", () => {
    expect(resolveScrollTopAfterHistoryPrepend(400, 12, 900)).toBe(512);
    expect(resolveScrollTopAfterHistoryPrepend(400, 0, 900)).toBe(500);
    expect(resolveScrollTopAfterHistoryPrepend(400, 12, 350)).toBe(12);
  });
});

function createHistory(options: {
  hasMoreSegments: boolean;
  lines?: string[];
}): HistoricalPane {
  return {
    sessionId: "session-1",
    paneId: "pane-1",
    sourceSessionId: "session-1",
    sourcePaneId: "pane-1",
    source: "v2_pane_history",
    replayStrategy: "raw_vt_stream",
    restoreGuaranteeLevel: "basic_history",
    lines: options.lines ?? ["old command", "old output"],
    capturedAtMs: 1000n,
    hasGaps: false,
    hasMoreSegments: options.hasMoreSegments,
    fromEventSeq: 1n,
    nextEventSeq: options.hasMoreSegments ? 2n : null,
    segmentCount: 1,
    loadedPayloadBytes: 32n,
  };
}

function createScreen(lines: string[]): FocusedScreen {
  return {
    cols: 96,
    pane_id: "pane-1",
    rows: 24,
    sequence: 7n,
    source: "native_emulator",
    surface: {
      cursor: null,
      lines: lines.map((text) => ({ text })),
      title: "Shell",
    },
  };
}
