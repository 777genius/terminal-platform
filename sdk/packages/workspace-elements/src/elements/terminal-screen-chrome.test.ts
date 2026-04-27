import { describe, expect, it } from "vitest";
import type { WorkspaceSnapshot } from "@terminal-platform/workspace-core";

import {
  resolveTerminalScreenChromeState,
  TERMINAL_SCREEN_CHROME_MODES,
} from "./terminal-screen-chrome.js";

type FocusedScreen = NonNullable<NonNullable<WorkspaceSnapshot["attachedSession"]>["focused_screen"]>;

describe("terminal screen chrome", () => {
  it("keeps full chrome labels explicit for panel surfaces", () => {
    const state = resolveTerminalScreenChromeState(createScreen(), {
      fontScale: "default",
      lineWrap: true,
    });

    expect(state.mode).toBe(TERMINAL_SCREEN_CHROME_MODES.full);
    expect(state.title).toBe("Shell");
    expect(state.metaItems.map((item) => item.label)).toEqual([
      "96 columns",
      "24 rows",
      "seq 7",
      "native_emulator",
      "default",
      "wrapped",
      "cursor 2:8",
    ]);
  });

  it("uses compact terminal labels for dense terminal placement chrome", () => {
    const state = resolveTerminalScreenChromeState(
      createScreen({ title: "  " }),
      {
        fontScale: "compact",
        lineWrap: false,
      },
      { mode: TERMINAL_SCREEN_CHROME_MODES.compact },
    );

    expect(state.mode).toBe(TERMINAL_SCREEN_CHROME_MODES.compact);
    expect(state.title).toBe("Live output");
    expect(state.metaItems.map((item) => [item.id, item.label])).toEqual([
      ["size", "96x24"],
      ["source", "native_emulator"],
      ["sequence", "seq 7"],
      ["fontScale", "compact"],
      ["wrap", "nowrap"],
      ["cursor", "2:8"],
    ]);
  });
});

function createScreen(options: { title?: string | null } = {}): FocusedScreen {
  return {
    cols: 96,
    pane_id: "pane-main",
    rows: 24,
    sequence: 7n,
    source: "native_emulator",
    surface: {
      cursor: { col: 7, row: 1 },
      lines: [],
      title: options.title ?? "Shell",
    },
  };
}
