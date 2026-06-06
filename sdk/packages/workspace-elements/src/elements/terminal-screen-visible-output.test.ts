import { describe, expect, it } from "vitest";

import type { WorkspaceSnapshot } from "@terminal-platform/workspace-core";

import {
  createVisibleOutputLines,
  resolveScrollTopAfterHistoryPrepend,
  shouldAutoLoadMoreHistoryFromViewport,
} from "./terminal-screen-element.js";

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

  it("marks partial restored history when the first loaded page has no visible lines", () => {
    const lines = createVisibleOutputLines(
      createHistory({ hasMoreSegments: true, lines: [] }),
      createScreen(["live prompt"]),
    );

    expect(lines).toEqual([
      {
        text: "--- restored history is partial; more persisted output is available ---",
        source: "boundary",
      },
      { text: "--- restored history above; live process below ---", source: "boundary" },
      { text: "live prompt", source: "live" },
    ]);
  });

  it("auto-loads older history only near the top while idle", () => {
    expect(shouldAutoLoadMoreHistoryFromViewport(
      { scrollTop: 0 } as HTMLElement,
      true,
      "idle",
    )).toBe(true);
    expect(shouldAutoLoadMoreHistoryFromViewport(
      { scrollTop: 24 } as HTMLElement,
      true,
      "idle",
    )).toBe(true);
    expect(shouldAutoLoadMoreHistoryFromViewport(
      { scrollTop: 25 } as HTMLElement,
      true,
      "idle",
    )).toBe(false);
    expect(shouldAutoLoadMoreHistoryFromViewport(
      { scrollTop: 0 } as HTMLElement,
      false,
      "idle",
    )).toBe(false);
    expect(shouldAutoLoadMoreHistoryFromViewport(
      { scrollTop: 0 } as HTMLElement,
      true,
      "loading",
    )).toBe(false);
  });

  it("preserves the viewport anchor after prepending older history", () => {
    expect(resolveScrollTopAfterHistoryPrepend(400, 12, 900)).toBe(512);
    expect(resolveScrollTopAfterHistoryPrepend(400, 0, 900)).toBe(500);
    expect(resolveScrollTopAfterHistoryPrepend(400, 12, 350)).toBe(12);
  });
});

function createHistory(options: { hasMoreSegments: boolean; lines?: string[] }): HistoricalPane {
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
