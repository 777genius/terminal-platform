import { describe, expect, it } from "vitest";

import type {
  ScreenCursor,
  ScreenLineMedia,
  ScreenLineSemanticMark,
  ScreenLineSideEffect,
  ScreenLineSpan,
  ScreenSurfacePalette,
  ScreenTextStyle,
} from "@terminal-platform/runtime-types";
import type { WorkspaceSnapshot } from "@terminal-platform/workspace-core";

import {
  createTerminalCommandContextCopyText,
  createTerminalCommandLineRichText,
  createTerminalHistoryEntries,
  createTerminalOutputAutolinkRuns,
  createTerminalOutputStyledSearchRuns,
  createVisibleOutputLines,
  doesCommandPresentationMatchHistoryEntry,
  normalizeTerminalBellCount,
  normalizeTerminalHyperlink,
  resolveTerminalCursorTextIndex,
  prepareTerminalHistoryEntriesForRender,
  terminalOutputMediaDataUri,
  terminalOutputMediaImageStyle,
  terminalOutputSideEffectLabel,
  terminalOutputSideEffectTitle,
  resolveTerminalOutputStyle,
  resolveTerminalSurfacePaletteStyle,
  resolveScrollTopAfterHistoryPrepend,
  shouldAutoLoadMoreHistoryFromViewport,
  shouldTriggerTerminalVisualBell,
  terminalColorToCss,
  terminalOutputMediaTitle,
} from "./terminal-screen-element.js";

type FocusedScreen = NonNullable<
  WorkspaceSnapshot["attachedSession"]
>["focused_screen"];
type HistoricalPane = NonNullable<WorkspaceSnapshot["historicalPanes"]>[string];

describe("terminal screen visible output", () => {
  it("maps terminal colors and text attributes to themed CSS styles", () => {
    expect(terminalColorToCss({ kind: "named", name: "red" })).toBe("#ef4444");
    expect(terminalColorToCss({ kind: "named", name: "bright_red" })).toBe(
      "#f87171"
    );
    expect(terminalColorToCss({ kind: "named", name: "bright_blue" })).toBe(
      "#60a5fa"
    );
    expect(terminalColorToCss({ kind: "named", name: "Bright Red" })).toBe(
      "#f87171"
    );
    expect(terminalColorToCss({ kind: "named", name: "bright-blue" })).toBe(
      "#60a5fa"
    );
    expect(terminalColorToCss({ kind: "named", name: "brightRed" })).toBe(
      "#f87171"
    );
    expect(terminalColorToCss({ kind: "named", name: "dimForeground" })).toBe(
      "var(--tp-terminal-color-text-muted)"
    );
    expect(terminalColorToCss({ kind: "named", name: "light blue" })).toBe(
      "rgb(173 216 230)"
    );
    expect(terminalColorToCss({ kind: "named", name: "light-blue" })).toBe(
      "rgb(173 216 230)"
    );
    expect(terminalColorToCss({ kind: "named", name: "slate_grey" })).toBe(
      "rgb(112 128 144)"
    );
    expect(terminalColorToCss({ kind: "named", name: "tomato" })).toBe(
      "rgb(255 99 71)"
    );
    expect(terminalColorToCss({ kind: "named", name: "orchid" })).toBe(
      "rgb(218 112 214)"
    );
    expect(terminalColorToCss({ kind: "named", name: "dodger blue" })).toBe(
      "rgb(30 144 255)"
    );
    expect(terminalColorToCss({ kind: "named", name: "light sea green" })).toBe(
      "rgb(32 178 170)"
    );
    expect(terminalColorToCss({ kind: "named", name: "LightSeaGreen" })).toBe(
      "rgb(32 178 170)"
    );
    expect(terminalColorToCss({ kind: "named", name: "dark-slate-gray" })).toBe(
      "rgb(47 79 79)"
    );
    expect(terminalColorToCss({ kind: "named", name: "rebecca_purple" })).toBe(
      "rgb(102 51 153)"
    );
    expect(terminalColorToCss({ kind: "named", name: "blanched almond" })).toBe(
      "rgb(255 235 205)"
    );
    expect(terminalColorToCss({ kind: "named", name: "gray" })).toBe(
      "rgb(128 128 128)"
    );
    expect(terminalColorToCss({ kind: "named", name: "gray90" })).toBe(
      "rgb(230 230 230)"
    );
    expect(terminalColorToCss({ kind: "named", name: "grey-50" })).toBe(
      "rgb(128 128 128)"
    );
    expect(
      terminalColorToCss({ kind: "named", name: "gray101" })
    ).toBeUndefined();
    expect(terminalColorToCss({ kind: "rgb", r: 12, g: 34, b: 56 })).toBe(
      "rgb(12 34 56)"
    );
    expect(terminalColorToCss({ kind: "rgb", r: -1, g: 260, b: 42.8 })).toBe(
      "rgb(0 255 42)"
    );
    expect(
      terminalColorToCss({ kind: "rgb", r: Number.NaN, g: 0, b: 0 })
    ).toBeUndefined();
    expect(terminalColorToCss({ kind: "indexed", index: 0 })).toBe("#111827");
    expect(terminalColorToCss({ kind: "indexed", index: 15 })).toBe("#f9fafb");
    expect(terminalColorToCss({ kind: "indexed", index: 16 })).toBe(
      "rgb(0 0 0)"
    );
    expect(terminalColorToCss({ kind: "indexed", index: 22 })).toBe(
      "rgb(0 95 0)"
    );
    expect(terminalColorToCss({ kind: "indexed", index: 231 })).toBe(
      "rgb(255 255 255)"
    );
    expect(terminalColorToCss({ kind: "indexed", index: 232 })).toBe(
      "rgb(8 8 8)"
    );
    expect(terminalColorToCss({ kind: "indexed", index: 255 })).toBe(
      "rgb(238 238 238)"
    );
    expect(terminalColorToCss({ kind: "indexed", index: 256 })).toBeUndefined();
    expect(
      terminalColorToCss({ kind: "indexed", index: Number.POSITIVE_INFINITY })
    ).toBeUndefined();

    expect(
      resolveTerminalOutputStyle(
        terminalStyle({
          background: { kind: "indexed", index: 22 },
          foreground: { kind: "named", name: "red" },
          inverse: true,
          underline: "curly",
          underline_color: { kind: "rgb", r: 1, g: 2, b: 3 },
        })
      )
    ).toMatchObject({
      backgroundColor: "#ef4444",
      color: "rgb(0 95 0)",
      textDecorationColor: "rgb(1 2 3)",
      textDecorationLine: "underline",
      textDecorationStyle: "wavy",
    });

    expect(
      resolveTerminalOutputStyle(
        terminalStyle({
          inverse: true,
        })
      )
    ).toMatchObject({
      backgroundColor:
        "var(--tp-terminal-surface-foreground-color, var(--tp-terminal-color-text))",
      color:
        "var(--tp-terminal-surface-background-color, var(--tp-terminal-color-bg))",
    });

    expect(
      resolveTerminalOutputStyle(
        terminalStyle({
          bold: true,
          dim: true,
          hidden: true,
          italic: true,
          blink: true,
          overline: true,
          border: "encircled",
          strikethrough: true,
          underline: "dashed",
        })
      )
    ).toMatchObject({
      animation: "terminal-output-blink 1s steps(1, end) infinite",
      color: "transparent",
      fontStyle: "italic",
      fontWeight: "760",
      opacity: "0.72",
      outline: "1px solid currentColor",
      outlineOffset: "-1px",
      borderRadius: "999px",
      textDecorationLine: "underline overline line-through",
      textDecorationStyle: "dashed",
      textShadow: "none",
    });

    expect(
      resolveTerminalOutputStyle(
        terminalStyle({
          border: "framed",
        })
      )
    ).toMatchObject({
      borderRadius: "0.12rem",
      outline: "1px solid currentColor",
    });

    expect(
      resolveTerminalOutputStyle(
        terminalStyle({
          baseline: "superscript",
        })
      )
    ).toMatchObject({
      fontSize: "0.78em",
      verticalAlign: "super",
    });

    expect(
      resolveTerminalOutputStyle(
        terminalStyle({
          baseline: "subscript",
        })
      )
    ).toMatchObject({
      fontSize: "0.78em",
      verticalAlign: "sub",
    });

    expect(
      resolveTerminalOutputStyle(
        terminalStyle({
          underline: "double",
        })
      )
    ).toMatchObject({
      textDecorationLine: "underline",
      textDecorationStyle: "double",
    });

    expect(
      resolveTerminalOutputStyle(
        terminalStyle({
          underline: "dotted",
        })
      )
    ).toMatchObject({
      textDecorationLine: "underline",
      textDecorationStyle: "dotted",
    });
  });

  it("maps terminal surface palette overrides to viewport CSS styles", () => {
    expect(
      resolveTerminalSurfacePaletteStyle({
        foreground: { kind: "rgb", r: 1, g: 2, b: 3 },
        background: { kind: "rgb", r: 4, g: 5, b: 6 },
        cursor: { kind: "rgb", r: 7, g: 8, b: 9 },
      })
    ).toMatchObject({
      background: "rgb(4 5 6)",
      caretColor: "rgb(7 8 9)",
      color: "rgb(1 2 3)",
      "--tp-terminal-surface-background-color": "rgb(4 5 6)",
      "--tp-terminal-surface-cursor-color": "rgb(7 8 9)",
      "--tp-terminal-surface-foreground-color": "rgb(1 2 3)",
    });
  });

  it("maps mixed rich terminal visual styles to CSS without losing color intent", () => {
    expect(
      resolveTerminalOutputStyle(
        terminalStyle({
          background: { kind: "rgb", r: 4, g: 5, b: 6 },
          foreground: { kind: "indexed", index: 22 },
          inverse: true,
          overline: true,
          strikethrough: true,
          underline: "curly",
          underline_color: { kind: "rgb", r: 9, g: 8, b: 7 },
        })
      )
    ).toMatchObject({
      backgroundColor: "rgb(0 95 0)",
      color: "rgb(4 5 6)",
      textDecorationColor: "rgb(9 8 7)",
      textDecorationLine: "underline overline line-through",
      textDecorationStyle: "wavy",
    });

    expect(
      resolveTerminalOutputStyle(
        terminalStyle({
          background: { kind: "rgb", r: 4, g: 5, b: 6 },
          foreground: { kind: "named", name: "bright-magenta" },
          hidden: true,
        })
      )
    ).toMatchObject({
      backgroundColor: "rgb(4 5 6)",
      color: "transparent",
      textShadow: "none",
    });
  });

  it("uses terminal surface palette as the inverse video fallback", () => {
    expect(
      resolveTerminalOutputStyle(
        terminalStyle({
          inverse: true,
        })
      )
    ).toMatchObject({
      backgroundColor:
        "var(--tp-terminal-surface-foreground-color, var(--tp-terminal-color-text))",
      color:
        "var(--tp-terminal-surface-background-color, var(--tp-terminal-color-bg))",
    });
  });

  it("preserves terminal surface palette on restored history lines", () => {
    const surfacePalette: ScreenSurfacePalette = {
      foreground: { kind: "rgb", r: 1, g: 2, b: 3 },
      background: { kind: "rgb", r: 4, g: 5, b: 6 },
      cursor: { kind: "rgb", r: 7, g: 8, b: 9 },
    };
    const lines = createVisibleOutputLines(
      createHistory({
        hasMoreSegments: false,
        lines: ["plain"],
        surfacePalette,
      }),
      createScreen([])
    );

    expect(lines).toEqual([
      {
        text: "plain",
        source: "history",
        palette: surfacePalette,
      },
    ]);
  });

  it("preserves terminal surface palette through command history grouping", () => {
    const surfacePalette: ScreenSurfacePalette = {
      foreground: { kind: "rgb", r: 1, g: 2, b: 3 },
      background: { kind: "rgb", r: 4, g: 5, b: 6 },
      cursor: { kind: "rgb", r: 7, g: 8, b: 9 },
    };
    const lines = createVisibleOutputLines(
      createHistory({
        hasMoreSegments: false,
        lines: ["shell % printf 123", "123"],
        surfacePalette,
      }),
      createScreen([])
    );
    const entries = createTerminalHistoryEntries(lines);

    expect(entries).toEqual([
      {
        kind: "command",
        prompt: "shell",
        commandLine: {
          text: "shell % printf 123",
          source: "history",
          palette: surfacePalette,
        },
        commandLineIndex: 0,
        command: "printf 123",
        output: [
          {
            line: {
              text: "123",
              source: "history",
              palette: surfacePalette,
            },
            lineIndex: 1,
          },
        ],
      },
    ]);
  });

  it("preserves live terminal cursor metadata and empty cursor rows", () => {
    expect(
      createVisibleOutputLines(
        createHistory({ hasMoreSegments: false, lines: [] }),
        createScreen(["ready"], {
          cursor: { row: 0, col: 2, shape: "beam", blinking: true },
        })
      )
    ).toEqual([
      {
        text: "ready",
        source: "live",
        cursor: { col: 2, shape: "beam", blinking: true },
      },
    ]);

    expect(
      createVisibleOutputLines(
        null,
        createScreen([], {
          cursor: { row: 2, col: 0, shape: "underline" },
        })
      )
    ).toEqual([
      { text: "", source: "live" },
      { text: "", source: "live" },
      {
        text: "",
        source: "live",
        cursor: { col: 0, shape: "underline" },
      },
    ]);
  });

  it("keeps hidden cursors out of visible output", () => {
    expect(
      createVisibleOutputLines(
        null,
        createScreen(["ready"], {
          cursor: { row: 0, col: 2, shape: "hidden", blinking: true },
        })
      )
    ).toEqual([{ text: "ready", source: "live" }]);
  });

  it("maps terminal cursor columns across wide and combining characters", () => {
    expect(resolveTerminalCursorTextIndex("abc", 2)).toBe(2);
    expect(resolveTerminalCursorTextIndex("表A", 1)).toBe(0);
    expect(resolveTerminalCursorTextIndex("表A", 2)).toBe(1);
    expect(resolveTerminalCursorTextIndex(`e${"\u0301"}A`, 1)).toBe(2);
    expect(resolveTerminalCursorTextIndex("🧪A", 1)).toBe(0);
    expect(resolveTerminalCursorTextIndex("🧪A", 2)).toBe("🧪".length);
    expect(resolveTerminalCursorTextIndex("👩‍💻A", 2)).toBe("👩‍💻".length);
    expect(resolveTerminalCursorTextIndex("❤️A", 2)).toBe("❤️".length);
    expect(resolveTerminalCursorTextIndex("🇺🇦A", 2)).toBe("🇺🇦".length);
  });

  it("normalizes terminal bell counts for visual bell rendering", () => {
    expect(normalizeTerminalBellCount(undefined)).toBe(0);
    expect(normalizeTerminalBellCount(null)).toBe(0);
    expect(normalizeTerminalBellCount(Number.NaN)).toBe(0);
    expect(normalizeTerminalBellCount(-1)).toBe(0);
    expect(normalizeTerminalBellCount(2.8)).toBe(2);
    expect(normalizeTerminalBellCount(-3n)).toBe(0);
    expect(normalizeTerminalBellCount(3n)).toBe(3);
  });

  it("triggers visual bell only for count increases on the same pane", () => {
    expect(shouldTriggerTerminalVisualBell(null, null, "pane-1", 1)).toBe(
      false
    );
    expect(shouldTriggerTerminalVisualBell("pane-1", 1, "pane-1", 1)).toBe(
      false
    );
    expect(shouldTriggerTerminalVisualBell("pane-1", 1, "pane-1", 2)).toBe(
      true
    );
    expect(shouldTriggerTerminalVisualBell("pane-1", 2, "pane-1", 1)).toBe(
      false
    );
    expect(shouldTriggerTerminalVisualBell("pane-1", 1, "pane-2", 2)).toBe(
      false
    );
    expect(shouldTriggerTerminalVisualBell("pane-1", 1, null, 2)).toBe(false);
  });

  it("keeps terminal media markers visible even when the text line is empty", () => {
    expect(
      createVisibleOutputLines(
        null,
        createScreen([
          {
            text: "",
            media: [
              {
                kind: "iterm2_image",
                name: "tiny.png",
                inline: true,
                mime_type: "image/png",
                data_base64: "iVBORw0KGgo=",
              },
            ],
          },
        ])
      )
    ).toEqual([
      {
        text: "",
        media: [
          {
            kind: "iterm2_image",
            name: "tiny.png",
            inline: true,
            mime_type: "image/png",
            data_base64: "iVBORw0KGgo=",
          },
        ],
        source: "live",
      },
    ]);
  });

  it("creates safe inline terminal media previews only for allowed image payloads", () => {
    const pngMedia: ScreenLineMedia = {
      kind: "kitty_graphics",
      inline: true,
      mime_type: "image/png",
      data_base64: "iVBORw0KGgo=",
      width: "6000px",
      height: "150%",
      preserve_aspect_ratio: false,
    };

    expect(terminalOutputMediaDataUri(pngMedia)).toBe(
      "data:image/png;base64,iVBORw0KGgo="
    );
    expect(
      terminalOutputMediaDataUri({
        ...pngMedia,
        mime_type: " IMAGE/PNG ",
        data_base64: "iVBORw0K\nGgo=",
      })
    ).toBe("data:image/png;base64,iVBORw0KGgo=");
    expect(terminalOutputMediaTitle({ ...pngMedia, byte_size: 12.8 })).toBe(
      "Kitty graphics sequence received: image/png, 12 bytes"
    );
    expect(terminalOutputMediaTitle({ ...pngMedia, name: " graph.png " })).toBe(
      "Kitty graphics sequence received: graph.png, image/png"
    );
    expect(terminalOutputMediaTitle({ kind: "sixel" })).toBe(
      "Sixel graphic sequence received"
    );
    expect(
      terminalOutputMediaTitle({
        ...pngMedia,
        name: " graph\u0007one.png ",
        mime_type: " IMAGE/PNG ",
      })
    ).toBe("Kitty graphics sequence received: graph one.png, image/png");
    expect(
      terminalOutputMediaTitle({
        ...pngMedia,
        name: `${"x".repeat(220)}.png`,
      })
    ).toBe(`Kitty graphics sequence received: ${"x".repeat(160)}, image/png`);
    expect(
      terminalOutputMediaTitle({
        ...pngMedia,
        byte_size: Number.NaN,
        truncated: true,
      })
    ).toBe("Kitty graphics sequence received: image/png, truncated");
    expect(terminalOutputMediaImageStyle(pngMedia)).toBe(
      "width:4096px;height:100%;object-fit:fill"
    );
    expect(
      terminalOutputMediaImageStyle({
        ...pngMedia,
        width: "12",
        height: "5",
        preserve_aspect_ratio: true,
      })
    ).toBe("width:12ch;height:6em;object-fit:contain");
    expect(
      terminalOutputMediaImageStyle({
        ...pngMedia,
        width: "calc(100vw)",
        height: "none",
      })
    ).toBe("object-fit:fill");
    expect(
      terminalOutputMediaImageStyle({
        ...pngMedia,
        width: "0px",
        height: "0%",
      })
    ).toBe("object-fit:fill");
    expect(
      terminalOutputMediaDataUri({
        ...pngMedia,
        mime_type: "image/svg+xml",
      })
    ).toBe("");
    expect(
      terminalOutputMediaDataUri({
        ...pngMedia,
        data_base64: "iVBORw0KGgo=\n<script>",
      })
    ).toBe("");
    expect(
      terminalOutputMediaDataUri({
        ...pngMedia,
        inline: false,
      })
    ).toBe("");
  });

  it("keeps terminal side-effect markers visible even when the text line is empty", () => {
    const blockedNotification: ScreenLineSideEffect = {
      kind: "desktop_notification",
      disposition: "blocked",
      target: "desktop_notification",
      message: "Build finished",
    };

    expect(
      createVisibleOutputLines(
        null,
        createScreen([
          {
            text: "",
            side_effects: [blockedNotification],
          },
        ])
      )
    ).toEqual([
      {
        text: "",
        sideEffects: [blockedNotification],
        source: "live",
      },
    ]);
  });

  it("creates clear labels for blocked terminal side effects", () => {
    expect(
      terminalOutputSideEffectLabel({
        kind: "clipboard_write",
        disposition: "blocked",
        target: "clipboard",
      })
    ).toBe("Clipboard write blocked");
    expect(
      terminalOutputSideEffectLabel({
        kind: "clipboard_read",
        disposition: "blocked",
        target: "selection",
      })
    ).toBe("Clipboard read blocked");
    expect(
      terminalOutputSideEffectLabel({
        kind: "desktop_notification",
        disposition: "blocked",
        target: "desktop_notification",
      })
    ).toBe("Notification blocked");
    expect(
      terminalOutputSideEffectTitle({
        kind: "desktop_notification",
        disposition: "blocked",
        target: "desktop_notification",
        message: "Build\u0007finished",
      })
    ).toBe("Notification blocked: desktop notification: Build finished");
  });

  it("keeps terminal semantic marks visible even when the text line is empty", () => {
    const semanticMarks: ScreenLineSemanticMark[] = [
      {
        kind: "command_finished",
        col: 0,
        exit_code: 1,
      },
    ];

    expect(
      createVisibleOutputLines(
        null,
        createScreen([
          {
            text: "",
            semantic_marks: semanticMarks,
          },
        ])
      )
    ).toEqual([
      {
        text: "",
        semanticMarks,
        source: "live",
      },
    ]);
  });

  it("uses shell integration semantic marks to group command history", () => {
    const lines = createVisibleOutputLines(
      null,
      createScreen([
        {
          text: "λ git status",
          semantic_marks: [
            { kind: "prompt_start", col: 0 },
            { kind: "input_start", col: 2 },
          ],
        },
        {
          text: "On branch main",
          semantic_marks: [{ kind: "output_start", col: 0 }],
        },
        {
          text: "",
          semantic_marks: [{ kind: "command_finished", col: 0, exit_code: 0 }],
        },
      ])
    );
    const entries = createTerminalHistoryEntries(lines);

    expect(entries).toHaveLength(1);
    expect(entries[0]).toMatchObject({
      kind: "command",
      prompt: "λ",
      command: "git status",
      output: [{ line: { text: "On branch main" }, lineIndex: 1 }],
    });
  });

  it("keeps terminal hyperlinks explicit and rejects unsafe protocols", () => {
    expect(normalizeTerminalHyperlink(" https://example.com/path ")).toBe(
      "https://example.com/path"
    );
    expect(normalizeTerminalHyperlink("mailto:team@example.com")).toBe(
      "mailto:team@example.com"
    );
    expect(normalizeTerminalHyperlink("file:///tmp/report.txt")).toBe(
      "file:///tmp/report.txt"
    );
    expect(normalizeTerminalHyperlink("ftp://example.com/pub/log.txt")).toBe(
      "ftp://example.com/pub/log.txt"
    );
    expect(
      normalizeTerminalHyperlink("https://example.com/\u0007x")
    ).toBeNull();
    expect(normalizeTerminalHyperlink("javascript:alert(1)")).toBeNull();
    expect(normalizeTerminalHyperlink("../relative")).toBeNull();
  });

  it("detects safe plain output links without absorbing terminal punctuation", () => {
    expect(
      createTerminalOutputAutolinkRuns(
        "Docs: https://example.com/docs, mailto:team@example.com, file:///tmp/report.txt and ftp://example.com/pub/log.txt."
      )
    ).toEqual([
      { kind: "text", text: "Docs: " },
      {
        href: "https://example.com/docs",
        kind: "link",
        text: "https://example.com/docs",
      },
      { kind: "text", text: ", " },
      {
        href: "mailto:team@example.com",
        kind: "link",
        text: "mailto:team@example.com",
      },
      { kind: "text", text: ", " },
      {
        href: "file:///tmp/report.txt",
        kind: "link",
        text: "file:///tmp/report.txt",
      },
      { kind: "text", text: " and " },
      {
        href: "ftp://example.com/pub/log.txt",
        kind: "link",
        text: "ftp://example.com/pub/log.txt",
      },
      { kind: "text", text: "." },
    ]);
  });

  it("ignores unsafe or relative plain output link candidates", () => {
    expect(
      createTerminalOutputAutolinkRuns(
        "bad javascript:alert(1) relative ./file and safe http://127.0.0.1:3000/log"
      )
    ).toEqual([
      {
        kind: "text",
        text: "bad javascript:alert(1) relative ./file and safe ",
      },
      {
        href: "http://127.0.0.1:3000/log",
        kind: "link",
        text: "http://127.0.0.1:3000/log",
      },
    ]);
  });

  it("preserves terminal styles when search highlights rich output spans", () => {
    const promptStyle = terminalStyle({
      foreground: { kind: "named", name: "green" },
    });
    const outputStyle = terminalStyle({
      foreground: { kind: "rgb", r: 1, g: 2, b: 3 },
      hyperlink: "https://example.com/log",
      underline: "single",
    });
    const runs = createTerminalOutputStyledSearchRuns(
      {
        text: "ok failed",
        spans: [
          { text: "ok ", style: promptStyle },
          { text: "failed", style: outputStyle },
        ],
      },
      [
        { kind: "text", value: "ok " },
        { kind: "match", value: "failed", matchIndex: 0, active: true },
      ]
    );

    expect(runs).toEqual([
      {
        activeSearchMatch: false,
        searchMatch: false,
        style: promptStyle,
        text: "ok ",
      },
      {
        activeSearchMatch: true,
        searchMatch: true,
        style: outputStyle,
        text: "failed",
      },
    ]);
  });

  it("splits search highlights that cross styled span boundaries", () => {
    const redStyle = terminalStyle({
      foreground: { kind: "named", name: "red" },
    });
    const blueStyle = terminalStyle({
      foreground: { kind: "named", name: "blue" },
    });
    const runs = createTerminalOutputStyledSearchRuns(
      {
        text: "abcde",
        spans: [
          { text: "ab", style: redStyle },
          { text: "cde", style: blueStyle },
        ],
      },
      [
        { kind: "text", value: "a" },
        { kind: "match", value: "bcd", matchIndex: 0, active: false },
        { kind: "text", value: "e" },
      ]
    );

    expect(runs).toEqual([
      {
        activeSearchMatch: false,
        searchMatch: false,
        style: redStyle,
        text: "a",
      },
      {
        activeSearchMatch: false,
        searchMatch: true,
        style: redStyle,
        text: "b",
      },
      {
        activeSearchMatch: false,
        searchMatch: true,
        style: blueStyle,
        text: "cd",
      },
      {
        activeSearchMatch: false,
        searchMatch: false,
        style: blueStyle,
        text: "e",
      },
    ]);
  });

  it("falls back from styled search runs when text and spans disagree", () => {
    const style = terminalStyle();

    expect(
      createTerminalOutputStyledSearchRuns(
        { text: "actual", spans: [{ text: "actual", style }] },
        [{ kind: "match", value: "stale", matchIndex: 0, active: false }]
      )
    ).toBeNull();
    expect(
      createTerminalOutputStyledSearchRuns(
        { text: "actual", spans: [{ text: "stale", style }] },
        [{ kind: "match", value: "actual", matchIndex: 0, active: false }]
      )
    ).toBeNull();
  });

  it("preserves live rich output spans through command grouping", () => {
    const richSpans: ScreenLineSpan[] = [
      {
        text: "red",
        style: terminalStyle({
          foreground: { kind: "named", name: "red" },
          bold: true,
        }),
      },
    ];
    const lines = createVisibleOutputLines(
      createHistory({ hasMoreSegments: false, lines: [] }),
      createScreen([
        "shell % printf red",
        {
          text: "red",
          spans: richSpans,
        },
      ])
    );
    const entries = createTerminalHistoryEntries(lines);
    const commandEntry = entries.find((entry) => entry.kind === "command");

    expect(lines.at(-1)).toEqual({
      text: "red",
      source: "live",
      spans: richSpans,
    });
    expect(commandEntry?.kind).toBe("command");
    expect(commandEntry?.output[0]?.line.spans).toEqual(richSpans);
  });

  it("preserves rich command-line spans after command history grouping", () => {
    const promptStyle = terminalStyle({
      foreground: { kind: "named", name: "green" },
      dim: true,
    });
    const commandStyle = terminalStyle({
      foreground: { kind: "rgb", r: 245, g: 158, b: 11 },
      hyperlink: "https://example.com/command",
      underline: "single",
    });
    const commandSpans: ScreenLineSpan[] = [
      { text: "shell % ", style: promptStyle },
      { text: "printf ", style: commandStyle },
      { text: "color", style: { ...commandStyle, bold: true } },
    ];
    const entries = createTerminalHistoryEntries([
      {
        text: "shell % printf color",
        source: "live",
        spans: commandSpans,
      },
      { text: "color", source: "live" },
    ]);
    const commandEntry = entries.find((entry) => entry.kind === "command");

    if (!commandEntry || commandEntry.kind !== "command") {
      throw new Error("Expected a command history entry");
    }

    expect(createTerminalCommandLineRichText(commandEntry)).toEqual({
      source: "live",
      text: "printf color",
      spans: [
        { text: "printf ", style: commandStyle },
        { text: "color", style: { ...commandStyle, bold: true } },
      ],
    });
  });

  it("preserves styled trailing terminal cells", () => {
    const richSpans: ScreenLineSpan[] = [
      {
        text: "   ",
        style: terminalStyle({
          background: { kind: "named", name: "red" },
        }),
      },
    ];
    const lines = createVisibleOutputLines(
      createHistory({ hasMoreSegments: false, lines: [] }),
      createScreen([
        {
          text: "   ",
          spans: richSpans,
        },
      ])
    );

    expect(lines).toEqual([
      {
        text: "   ",
        source: "live",
        spans: richSpans,
      },
    ]);
  });

  it("preserves restored rich history spans for rendered snapshot history", () => {
    const richSpans: ScreenLineSpan[] = [
      {
        text: "red",
        style: terminalStyle({
          foreground: { kind: "named", name: "red" },
        }),
      },
    ];
    const lines = createVisibleOutputLines(
      createHistory({
        hasMoreSegments: false,
        lines: ["red"],
        richLines: [{ text: "red", spans: richSpans }],
      }),
      createScreen([])
    );

    expect(lines).toEqual([
      {
        text: "red",
        source: "history",
        spans: richSpans,
      },
    ]);
  });

  it("preserves soft-wrap metadata for live and restored history lines", () => {
    const liveLines = createVisibleOutputLines(
      createHistory({ hasMoreSegments: false, lines: [] }),
      createScreen([{ text: "wrapped live", wrapped: true }])
    );
    const historyLines = createVisibleOutputLines(
      createHistory({
        hasMoreSegments: false,
        lines: ["wrapped history"],
        richLines: [{ text: "wrapped history", spans: [], wrapped: true }],
      }),
      createScreen([])
    );

    expect(liveLines).toEqual([
      {
        text: "wrapped live",
        source: "live",
        softWrapped: true,
      },
    ]);
    expect(historyLines).toEqual([
      {
        text: "wrapped history",
        source: "history",
        softWrapped: true,
      },
    ]);
  });

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

  it("keeps restored rich history when overlapping live output has less render metadata", () => {
    const richSpans: ScreenLineSpan[] = [
      {
        text: "red",
        style: terminalStyle({
          foreground: { kind: "named", name: "red" },
        }),
      },
    ];
    const lines = createVisibleOutputLines(
      createHistory({
        hasMoreSegments: false,
        lines: ["red"],
        richLines: [{ text: "red", spans: richSpans }],
      }),
      createScreen(["red"])
    );

    expect(lines).toEqual([
      {
        text: "red",
        source: "history",
        spans: richSpans,
      },
      {
        text: "--- restored history above; live process below ---",
        source: "boundary",
      },
      { text: "red", source: "live" },
    ]);
  });

  it("dedupes restored plain spans against equivalent live text", () => {
    const plainSpans: ScreenLineSpan[] = [
      { text: "pla", style: terminalStyle() },
      { text: "in", style: terminalStyle() },
    ];
    const lines = createVisibleOutputLines(
      createHistory({
        hasMoreSegments: false,
        lines: ["plain"],
        richLines: [{ text: "plain", spans: plainSpans }],
      }),
      createScreen(["plain"])
    );

    expect(lines).toEqual([{ text: "plain", source: "live" }]);
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
        lines: ["<                      echo TP_VERIFY_1", "TP_VERIFY_1"],
      }),
      createScreen(["shell % echo TP_VERIFY_1", "TP_VERIFY_1"]),
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

    expect(lines).toEqual([{ text: "ok", source: "live" }]);
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

  it("creates command block context menu copy payloads", () => {
    const [entry] = createTerminalHistoryEntries([
      {
        text: "venv312 ~/dev/quanta % printf 'hello\\nworld\\n'",
        source: "live",
      },
      { text: "hello", source: "live" },
      { text: "world", source: "live" },
    ]);

    if (!entry || entry.kind !== "command") {
      throw new Error("Expected a command history entry");
    }

    expect(createTerminalCommandContextCopyText(entry)).toEqual({
      blockText: "printf 'hello\\nworld\\n'\nhello\nworld",
      commandText: "printf 'hello\\nworld\\n'",
      outputText: "hello\nworld",
    });
  });

  it("keeps styled blank terminal cells in command block copy payloads", () => {
    const styledBlankSpans: ScreenLineSpan[] = [
      {
        text: "   ",
        style: terminalStyle({
          background: { kind: "named", name: "green" },
        }),
      },
    ];
    const [entry] = createTerminalHistoryEntries([
      {
        text: "venv312 ~/dev/quanta % render blanks",
        source: "live",
      },
      {
        text: "   ",
        source: "live",
        spans: styledBlankSpans,
      },
    ]);

    if (!entry || entry.kind !== "command") {
      throw new Error("Expected a command history entry");
    }

    expect(createTerminalCommandContextCopyText(entry)).toEqual({
      blockText: "render blanks\n   ",
      commandText: "render blanks",
      outputText: "   ",
    });
  });

  it("renders a pending command block instead of a trailing raw command echo", () => {
    const entries = createTerminalHistoryEntries([
      {
        text: "tff",
        source: "live",
      },
    ]);
    const prepared = prepareTerminalHistoryEntriesForRender(
      entries,
      [
        {
          command: "tff",
          startedAtMs: 1_000,
          status: "running",
        },
      ],
      4_000
    );

    expect(prepared.entries).toEqual([
      {
        kind: "command",
        prompt: "shell",
        commandLine: {
          text: "shell % tff",
          source: "live",
        },
        commandLineIndex: 0,
        command: "tff",
        output: [],
      },
    ]);
    expect(prepared.metadataByEntryIndex.get(0)).toEqual({
      command: "tff",
      startedAtMs: 1_000,
      status: "running",
    });
  });

  it("promotes a raw command echo with its output before the active prompt cursor", () => {
    const entries = createTerminalHistoryEntries([
      {
        text: "print 'fdfd'",
        source: "live",
      },
      {
        text: "fdfd",
        source: "live",
      },
      {
        text: "custom prompt",
        source: "live",
        cursor: { col: 0, shape: "block" },
      },
    ]);
    const metadata = {
      command: "print 'fdfd'",
      durationMs: 35,
      startedAtMs: 1_000,
      status: "succeeded" as const,
    };

    const prepared = prepareTerminalHistoryEntriesForRender(
      entries,
      [metadata],
      4_000
    );

    expect(prepared.entries).toEqual([
      {
        kind: "command",
        prompt: "shell",
        commandLine: {
          text: "shell % print 'fdfd'",
          source: "live",
        },
        commandLineIndex: 0,
        command: "print 'fdfd'",
        output: [
          {
            line: { text: "fdfd", source: "live" },
            lineIndex: 1,
          },
        ],
      },
      {
        kind: "line",
        line: {
          text: "custom prompt",
          source: "live",
          cursor: { col: 0, shape: "block" },
        },
        lineIndex: 2,
      },
    ]);
    expect(prepared.metadataByEntryIndex.get(0)).toBe(metadata);
  });

  it("moves restored output onto the authoritative live command when the live screen lags", () => {
    const entries = createTerminalHistoryEntries([
      { text: "shell % nnot_a_real_command", source: "history" },
      {
        text: "zsh: command not found: not_a_real_command",
        source: "history",
      },
      { text: "shell % not_a_real_command", source: "live" },
    ]);
    const metadata = {
      command: "not_a_real_command",
      durationMs: 220,
      startedAtMs: 1_000,
      status: "failed" as const,
    };

    const prepared = prepareTerminalHistoryEntriesForRender(
      entries,
      [metadata],
      4_000
    );

    expect(prepared.entries).toEqual([
      {
        kind: "command",
        prompt: "shell",
        commandLine: {
          text: "shell % not_a_real_command",
          source: "live",
        },
        commandLineIndex: 2,
        command: "not_a_real_command",
        output: [
          {
            line: {
              text: "zsh: command not found: not_a_real_command",
              source: "history",
            },
            lineIndex: 1,
          },
        ],
      },
    ]);
    expect(prepared.metadataByEntryIndex.get(2)).toBe(metadata);
  });

  it("removes type-ahead command echoes from the previous command output", () => {
    const longCommand =
      "printf 'WRAP_OK_%s\\n' 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789'";
    const promptCommand = "printf 'PROMPT_DOLLAR $\\nNEXT\\n'";
    const rapidCommand = "printf 'RAPID\\n'";
    const entries = createTerminalHistoryEntries([
      { text: "shell % missing", source: "history" },
      { text: "zsh: command not found: missing", source: "history" },
      {
        text: "printf 'WRAP_OK_%s\\n' 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnop",
        source: "history",
        softWrapped: true,
      },
      { text: "qrstuvwxyz0123456789'", source: "history" },
      {
        text: "shell % printf 'WRAP_OK_%s\\n' 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdef",
        source: "history",
        softWrapped: true,
      },
      { text: "ghijklmnopqrstuvwxyz0123456789'", source: "history" },
      {
        text: "WRAP_OK_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
        source: "history",
      },
      { text: promptCommand, source: "history" },
      { text: rapidCommand, source: "history" },
      { text: `shell % ${promptCommand}`, source: "history" },
      { text: "PROMPT_DOLLAR $", source: "history" },
      { text: "NEXT", source: "history" },
      { text: rapidCommand, source: "history" },
      { text: `shell % ${rapidCommand}`, source: "history" },
      { text: "RAPID", source: "history" },
    ]);

    const prepared = prepareTerminalHistoryEntriesForRender(
      entries,
      ["missing", longCommand, promptCommand, rapidCommand].map((command) => ({
        command,
        status: "succeeded" as const,
      })),
      4_000
    );

    expect(
      prepared.entries.map((entry) =>
        entry.kind === "command"
          ? {
              command: entry.command,
              output: entry.output.map((item) => item.line.text),
            }
          : null
      )
    ).toEqual([
      {
        command: "missing",
        output: ["zsh: command not found: missing"],
      },
      {
        command: longCommand,
        output: [
          "WRAP_OK_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
        ],
      },
      {
        command: promptCommand,
        output: ["PROMPT_DOLLAR $", "NEXT"],
      },
      { command: rapidCommand, output: ["RAPID"] },
    ]);
  });

  it("keeps command-shaped output when no later authoritative entry exists", () => {
    const entries = createTerminalHistoryEntries([
      { text: "shell % echo command", source: "history" },
      { text: "printf 'later\\n'", source: "history" },
    ]);

    const prepared = prepareTerminalHistoryEntriesForRender(
      entries,
      [
        { command: "echo command", status: "succeeded" },
        { command: "printf 'later\\n'", status: "succeeded" },
      ],
      4_000
    );

    expect(prepared.entries).toMatchObject([
      {
        kind: "command",
        command: "echo command",
        output: [{ line: { text: "printf 'later\\n'" } }],
      },
    ]);
  });

  it("drops a corrupted restored wrapped fragment only when its output matches the live block", () => {
    const command =
      "printf 'WRAP_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789\\n'";
    const output =
      "WRAP_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    const entries = createTerminalHistoryEntries([
      {
        text: "shell % pprintf 'WRAP_ABCDEFGHIJKLMNOPQRSTUVWXYZ01234",
        source: "history",
      },
      { text: "5", source: "history" },
      {
        text: "56789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789\\n'",
        source: "history",
      },
      { text: output, source: "history" },
      {
        text: "shell % printf 'WRAP_ABCDEFGHIJKLMNOPQRSTUVWXYZ01234",
        source: "live",
        softWrapped: true,
      },
      {
        text: "56789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789\\n'",
        source: "live",
      },
      { text: output, source: "live" },
    ]);
    const metadata = {
      command,
      durationMs: 18,
      startedAtMs: 1_000,
      status: "succeeded" as const,
    };

    const prepared = prepareTerminalHistoryEntriesForRender(
      entries,
      [metadata],
      4_000
    );

    expect(prepared.entries).toHaveLength(1);
    expect(prepared.entries[0]).toMatchObject({
      kind: "command",
      command,
      commandLine: { source: "live" },
      output: [{ line: { source: "live", text: output } }],
    });
  });

  it("keeps a corrupted restored wrapped fragment when its output differs", () => {
    const entries = createTerminalHistoryEntries([
      { text: "shell % pprintf 'WRAP_ABC", source: "history" },
      { text: "restored-only", source: "history" },
      {
        text: "shell % printf 'WRAP_ABCDEF'",
        source: "live",
      },
      { text: "live-only", source: "live" },
    ]);

    const prepared = prepareTerminalHistoryEntriesForRender(
      entries,
      [
        {
          command: "printf 'WRAP_ABCDEF'",
          durationMs: 18,
          startedAtMs: 1_000,
          status: "succeeded",
        },
      ],
      4_000
    );

    expect(prepared.entries).toHaveLength(2);
    expect(
      prepared.entries.map((entry) =>
        entry.kind === "command" ? entry.commandLine.source : null
      )
    ).toEqual(["history", "live"]);
  });

  it("does not append stale or already matched running command metadata", () => {
    const entries = createTerminalHistoryEntries([
      {
        text: "shell % pnpm test",
        source: "live",
      },
      {
        text: "ok",
        source: "live",
      },
    ]);

    expect(
      prepareTerminalHistoryEntriesForRender(
        entries,
        [
          {
            command: "old command",
            startedAtMs: 1_000,
            status: "running",
          },
          {
            command: "pnpm test",
            startedAtMs: 125_000,
            status: "running",
          },
        ],
        125_500
      ).entries
    ).toEqual(entries);
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

  it("keeps normal user printf commands while hiding internal smoke commands", () => {
    const lines = createVisibleOutputLines(
      null,
      createScreen([
        'shell % printf "TP_INTERNAL_1\\n"',
        "TP_INTERNAL_1",
        'shell % printf "hello\\n"',
        "hello",
      ]),
      { hideShellPromptNoise: true, preserveShellPromptCommands: true }
    );

    expect(lines).toEqual([
      { text: "TP_INTERNAL_1", source: "live" },
      { text: 'shell % printf "hello\\n"', source: "live" },
      { text: "hello", source: "live" },
    ]);
  });

  it("groups terminal commands after prompt-only rows are removed from visible output", () => {
    const lines = createVisibleOutputLines(
      null,
      createScreen([
        "shell %",
        "shell % echo one",
        "one",
        "shell %",
        "shell % echo two",
        "two",
        "shell %",
      ]),
      { hideShellPromptNoise: true, preserveShellPromptCommands: true }
    );

    expect(createTerminalHistoryEntries(lines)).toEqual([
      {
        kind: "command",
        prompt: "shell",
        commandLine: { text: "shell % echo one", source: "live" },
        commandLineIndex: 0,
        command: "echo one",
        output: [{ line: { text: "one", source: "live" }, lineIndex: 1 }],
      },
      {
        kind: "command",
        prompt: "shell",
        commandLine: { text: "shell % echo two", source: "live" },
        commandLineIndex: 2,
        command: "echo two",
        output: [{ line: { text: "two", source: "live" }, lineIndex: 3 }],
      },
    ]);
  });

  it("joins soft-wrapped command input before grouping its output", () => {
    const entries = createTerminalHistoryEntries([
      {
        text: "shell % printf 'WRAP_ABCDEFGHIJKLMNOPQRSTUVWXYZ01234",
        source: "live",
        softWrapped: true,
      },
      {
        text: "56789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789\\n'",
        source: "live",
      },
      {
        text: "WRAP_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
        source: "live",
      },
    ]);

    expect(entries).toEqual([
      {
        kind: "command",
        prompt: "shell",
        commandLine: {
          text: "shell % printf 'WRAP_ABCDEFGHIJKLMNOPQRSTUVWXYZ01234",
          source: "live",
          softWrapped: true,
        },
        commandLineIndex: 0,
        command:
          "printf 'WRAP_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789\\n'",
        output: [
          {
            line: {
              text: "WRAP_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
              source: "live",
            },
            lineIndex: 2,
          },
        ],
      },
    ]);
  });

  it("joins a restored soft-wrapped command with its live continuation", () => {
    const entries = createTerminalHistoryEntries([
      {
        text: "shell % printf 'WRAP_ABCDEFGHIJKLMNOPQRSTUVWXYZ01234",
        source: "history",
        softWrapped: true,
      },
      {
        text: "56789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789\\n'",
        source: "live",
      },
      {
        text: "WRAP_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
        source: "live",
      },
    ]);

    expect(entries).toEqual([
      {
        kind: "command",
        prompt: "shell",
        commandLine: {
          text: "shell % printf 'WRAP_ABCDEFGHIJKLMNOPQRSTUVWXYZ01234",
          source: "history",
          softWrapped: true,
        },
        commandLineIndex: 0,
        command:
          "printf 'WRAP_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789\\n'",
        output: [
          {
            line: {
              text: "WRAP_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
              source: "live",
            },
            lineIndex: 2,
          },
        ],
      },
    ]);
  });

  it("keeps unrelated restored output as plain lines between command groups", () => {
    const entries = createTerminalHistoryEntries([
      { text: "shell % echo before", source: "history" },
      { text: "before", source: "history" },
      { text: "standalone restored note", source: "history" },
      { text: "shell % echo after", source: "history" },
      { text: "after", source: "history" },
    ]);

    expect(entries).toEqual([
      {
        kind: "command",
        prompt: "shell",
        commandLine: { text: "shell % echo before", source: "history" },
        commandLineIndex: 0,
        command: "echo before",
        output: [
          { line: { text: "before", source: "history" }, lineIndex: 1 },
          {
            line: { text: "standalone restored note", source: "history" },
            lineIndex: 2,
          },
        ],
      },
      {
        kind: "command",
        prompt: "shell",
        commandLine: { text: "shell % echo after", source: "history" },
        commandLineIndex: 3,
        command: "echo after",
        output: [{ line: { text: "after", source: "history" }, lineIndex: 4 }],
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

  it("prefers live command output when restored history duplicated its first input character", () => {
    const entries = createTerminalHistoryEntries([
      { text: "shell % pprint 'fdfd'", source: "history" },
      { text: "fdfd", source: "history" },
      { text: "shell % print 'fdfd'", source: "live" },
      { text: "fdfd", source: "live" },
    ]);

    expect(entries).toEqual([
      {
        kind: "command",
        prompt: "shell",
        commandLine: {
          text: "shell % print 'fdfd'",
          source: "live",
        },
        commandLineIndex: 2,
        command: "print 'fdfd'",
        output: [
          {
            line: { text: "fdfd", source: "live" },
            lineIndex: 3,
          },
        ],
      },
    ]);
  });

  it("keeps a similar restored command when its output differs from live output", () => {
    const entries = createTerminalHistoryEntries([
      { text: "shell % eecho ok", source: "history" },
      { text: "restored output", source: "history" },
      { text: "shell % echo ok", source: "live" },
      { text: "live output", source: "live" },
    ]);

    expect(entries).toHaveLength(2);
    expect(entries.map((entry) => entry.kind)).toEqual(["command", "command"]);
    expect(
      entries.map((entry) =>
        entry.kind === "command" ? entry.commandLine.source : null
      )
    ).toEqual(["history", "live"]);
  });

  it("prefers a restored rich command over a plain live duplicate", () => {
    const richSpans: ScreenLineSpan[] = [
      {
        text: "TP_VERIFY_RICH",
        style: terminalStyle({
          foreground: { kind: "named", name: "bright_red" },
        }),
      },
    ];
    const entries = createTerminalHistoryEntries([
      { text: "shell % echo TP_VERIFY_RICH", source: "history" },
      {
        text: "TP_VERIFY_RICH",
        source: "history",
        spans: richSpans,
      },
      { text: "shell % echo TP_VERIFY_RICH", source: "live" },
      { text: "TP_VERIFY_RICH", source: "live" },
    ]);

    expect(entries).toHaveLength(1);
    expect(entries[0]).toMatchObject({
      kind: "command",
      commandLine: {
        text: "shell % echo TP_VERIFY_RICH",
        source: "history",
      },
      output: [
        {
          line: {
            text: "TP_VERIFY_RICH",
            source: "history",
            spans: richSpans,
          },
        },
      ],
    });
  });

  it("prefers a live rich command when restored and live output are visually equivalent", () => {
    const historySpans: ScreenLineSpan[] = [
      {
        text: "RED",
        style: terminalStyle({ foreground: { kind: "named", name: "red" } }),
      },
    ];
    const liveSpans: ScreenLineSpan[] = [
      {
        text: "RED",
        style: terminalStyle({ foreground: { name: "red", kind: "named" } }),
      },
    ];
    const entries = createTerminalHistoryEntries([
      { text: "shell % print RED", source: "history", softWrapped: true },
      { text: "", source: "history" },
      { text: "RED", source: "history", spans: historySpans },
      { text: "shell % print RED", source: "live", softWrapped: true },
      { text: "", source: "live" },
      { text: "RED", source: "live", spans: liveSpans },
    ]);

    expect(entries).toHaveLength(1);
    expect(entries[0]).toMatchObject({
      kind: "command",
      command: "print RED",
      commandLine: { source: "live" },
      output: [{ line: { source: "live", text: "RED" } }],
    });
  });

  it("drops command-only live duplicates when restored history already has command output", () => {
    const entries = createTerminalHistoryEntries([
      { text: "~/dev/project % pp", source: "history" },
      { text: "zsh: command not found: pp", source: "history" },
      { text: "~/dev/project (1.28s) % pp", source: "live" },
      { text: "", source: "live" },
    ]);

    expect(entries).toEqual([
      {
        kind: "command",
        prompt: "~/dev/project",
        commandLine: {
          text: "~/dev/project % pp",
          source: "history",
        },
        commandLineIndex: 0,
        command: "pp",
        output: [
          {
            line: { text: "zsh: command not found: pp", source: "history" },
            lineIndex: 1,
          },
        ],
      },
    ]);
  });

  it("treats styled blank command output as meaningful restored history", () => {
    const styledBlankSpans: ScreenLineSpan[] = [
      {
        text: "   ",
        style: terminalStyle({
          background: { kind: "named", name: "bright_blue" },
        }),
      },
    ];
    const entries = createTerminalHistoryEntries([
      { text: "~/dev/project % render blocks", source: "history" },
      { text: "   ", source: "history", spans: styledBlankSpans },
      { text: "~/dev/project (0.10s) % render blocks", source: "live" },
      { text: "", source: "live" },
    ]);

    expect(entries).toEqual([
      {
        kind: "command",
        prompt: "~/dev/project",
        commandLine: {
          text: "~/dev/project % render blocks",
          source: "history",
        },
        commandLineIndex: 0,
        command: "render blocks",
        output: [
          {
            line: {
              text: "   ",
              source: "history",
              spans: styledBlankSpans,
            },
            lineIndex: 1,
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

  it("drops no-output command fragments when a later full command block has output", () => {
    expect(
      createTerminalHistoryEntries([
        {
          text: "(venv312) shell % ls __tp_missing_",
          source: "live",
        },
        {
          text: "~/dev/project % ls __tp_missing_1781452725003",
          source: "live",
        },
        {
          text: "ls: __tp_missing_1781452725003: No such file or directory",
          source: "live",
        },
      ])
    ).toEqual([
      {
        kind: "command",
        prompt: "~/dev/project",
        commandLine: {
          text: "~/dev/project % ls __tp_missing_1781452725003",
          source: "live",
        },
        commandLineIndex: 1,
        command: "ls __tp_missing_1781452725003",
        output: [
          {
            line: {
              text: "ls: __tp_missing_1781452725003: No such file or directory",
              source: "live",
            },
            lineIndex: 2,
          },
        ],
      },
    ]);
  });

  it("matches command presentation metadata against wrapped command prefixes", () => {
    expect(
      doesCommandPresentationMatchHistoryEntry(
        "echo TP_LONG_VERIFY_1781453698047_ABCDEFGHIJKLMNOPQRSTU",
        "echo TP_LONG_VERIFY_1781453698047_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ"
      )
    ).toBe(true);
    expect(doesCommandPresentationMatchHistoryEntry("git", "git status")).toBe(
      false
    );
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
  richLines?: HistoricalPane["richLines"];
  surfacePalette?: HistoricalPane["surfacePalette"];
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
    ...(options.richLines ? { richLines: options.richLines } : {}),
    ...(options.surfacePalette
      ? { surfacePalette: options.surfacePalette }
      : {}),
    capturedAtMs: 1000n,
    hasGaps: false,
    hasMoreSegments: options.hasMoreSegments,
    fromEventSeq: 1n,
    nextEventSeq: options.hasMoreSegments ? 2n : null,
    segmentCount: 1,
    loadedPayloadBytes: 32n,
  };
}

type ScreenLineFixture =
  | string
  | {
      media?: ScreenLineMedia[];
      semantic_marks?: ScreenLineSemanticMark[];
      side_effects?: ScreenLineSideEffect[];
      spans?: ScreenLineSpan[];
      text: string;
      wrapped?: boolean;
    };

function createScreen(
  lines: ScreenLineFixture[],
  options: { bellCount?: bigint | number; cursor?: ScreenCursor | null } = {}
): FocusedScreen {
  return {
    cols: 96,
    pane_id: "pane-1",
    rows: 24,
    sequence: 7n,
    source: "native_emulator",
    surface: {
      cursor: options.cursor ?? null,
      ...(options.bellCount ? { bell_count: options.bellCount } : {}),
      lines: lines.map((line) =>
        typeof line === "string" ? { text: line } : line
      ),
      title: "Shell",
    },
  };
}

function terminalStyle(
  overrides: Partial<ScreenTextStyle> = {}
): ScreenTextStyle {
  return {
    foreground: null,
    background: null,
    underline_color: null,
    bold: false,
    dim: false,
    italic: false,
    blink: false,
    underline: null,
    overline: false,
    border: null,
    inverse: false,
    hidden: false,
    strikethrough: false,
    hyperlink: null,
    ...overrides,
  };
}
