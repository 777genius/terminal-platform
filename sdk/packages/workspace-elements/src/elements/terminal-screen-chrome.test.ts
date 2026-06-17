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
      "cursor 2:8 beam blinking",
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
      ["cursor", "2:8 beam blinking"],
    ]);
  });

  it("shows OSC 7 working directory metadata when present", () => {
    const state = resolveTerminalScreenChromeState(createScreen({
      workingDirectoryUri: "file://localhost/tmp/dev%20space",
    }), {
      fontScale: "default",
      lineWrap: true,
    });

    expect(state.metaItems.at(-1)).toEqual({
      id: "workingDirectory",
      label: "cwd /tmp/dev space",
      title: "file://localhost/tmp/dev%20space",
    });
  });

  it("shows terminal progress metadata when present", () => {
    const state = resolveTerminalScreenChromeState(
      createScreen({ progress: { state: "normal", value: 42 } }),
      {
        fontScale: "default",
        lineWrap: true,
      },
      { mode: TERMINAL_SCREEN_CHROME_MODES.compact },
    );

    expect(state.metaItems.find((item) => item.id === "progress")).toEqual({
      id: "progress",
      label: "42%",
      title: "Terminal progress 42%",
    });
  });

  it("shows warning and indeterminate terminal progress labels", () => {
    const warning = resolveTerminalScreenChromeState(
      createScreen({ progress: { state: "warning", value: 180 } }),
      {
        fontScale: "default",
        lineWrap: true,
      },
      { mode: TERMINAL_SCREEN_CHROME_MODES.compact },
    );
    const indeterminate = resolveTerminalScreenChromeState(
      createScreen({ progress: { state: "indeterminate" } }),
      {
        fontScale: "default",
        lineWrap: true,
      },
    );

    expect(warning.metaItems.find((item) => item.id === "progress")).toEqual({
      id: "progress",
      label: "warn 100%",
      title: "Terminal progress warning 100%",
    });
    expect(indeterminate.metaItems.find((item) => item.id === "progress")).toEqual({
      id: "progress",
      label: "progress pending",
      title: "Terminal progress indeterminate",
    });
  });
});

function createScreen(
  options: {
    title?: string | null;
    workingDirectoryUri?: string;
    progress?: FocusedScreen["surface"]["progress"];
  } = {},
): FocusedScreen {
  return {
    cols: 96,
    pane_id: "pane-main",
    rows: 24,
    sequence: 7n,
    source: "native_emulator",
    surface: {
      cursor: { blinking: true, col: 7, row: 1, shape: "beam" },
      lines: [],
      title: options.title ?? "Shell",
      ...(options.workingDirectoryUri
        ? { working_directory_uri: options.workingDirectoryUri }
        : {}),
      ...(options.progress ? { progress: options.progress } : {}),
    },
  };
}
