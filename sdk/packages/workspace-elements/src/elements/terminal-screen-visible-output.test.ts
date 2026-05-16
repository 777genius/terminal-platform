import { describe, expect, it } from "vitest";

import type { WorkspaceSnapshot } from "@terminal-platform/workspace-core";

import { createVisibleOutputLines } from "./terminal-screen-element.js";

type FocusedScreen = NonNullable<WorkspaceSnapshot["attachedSession"]>["focused_screen"];
type HistoricalPane = NonNullable<WorkspaceSnapshot["historicalPanes"]>[string];

describe("terminal screen visible output", () => {
  it("marks partial restored history before live output", () => {
    const lines = createVisibleOutputLines(
      createHistory({ hasMoreSegments: true }),
      createScreen(["live prompt"]),
    );

    expect(lines).toEqual([
      { text: "old command", source: "history" },
      { text: "old output", source: "history" },
      {
        text: "--- restored history is partial; more persisted output is available ---",
        source: "boundary",
      },
      { text: "--- restored history above; live process below ---", source: "boundary" },
      { text: "live prompt", source: "live" },
    ]);
  });

  it("marks partial restored history even when no live output is attached", () => {
    const lines = createVisibleOutputLines(
      createHistory({ hasMoreSegments: true }),
      createScreen([]),
    );

    expect(lines.at(-1)).toEqual({
      text: "--- restored history is partial; more persisted output is available ---",
      source: "boundary",
    });
  });
});

function createHistory(options: { hasMoreSegments: boolean }): HistoricalPane {
  return {
    sessionId: "session-1",
    paneId: "pane-1",
    sourceSessionId: "session-1",
    sourcePaneId: "pane-1",
    source: "v2_pane_history",
    replayStrategy: "raw_vt_stream",
    restoreGuaranteeLevel: "basic_history",
    lines: ["old command", "old output"],
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
