import { describe, expect, it } from "vitest";

import {
  resolveTerminalCommandComposerRowRange,
  resolveTerminalCommandComposerRows,
} from "./terminal-command-composer-layout.js";

describe("terminal command composer layout", () => {
  it("keeps empty and single-line commands compact", () => {
    expect(resolveTerminalCommandComposerRows("")).toBe(1);
    expect(resolveTerminalCommandComposerRows("git status")).toBe(1);
  });

  it("expands for multiline command drafts", () => {
    expect(resolveTerminalCommandComposerRows("printf one\nprintf two")).toBe(2);
    expect(resolveTerminalCommandComposerRows("one\r\ntwo\r\nthree")).toBe(3);
  });

  it("clamps rows between the configured minimum and maximum", () => {
    expect(resolveTerminalCommandComposerRows("one", { minRows: 2, maxRows: 4 })).toBe(2);
    expect(resolveTerminalCommandComposerRows("1\n2\n3\n4\n5\n6", { minRows: 2, maxRows: 4 })).toBe(4);
  });

  it("normalizes invalid row bounds conservatively", () => {
    expect(resolveTerminalCommandComposerRowRange({ minRows: 0, maxRows: -1 })).toEqual({
      minRows: 1,
      maxRows: 1,
    });
    expect(resolveTerminalCommandComposerRowRange({ minRows: 6, maxRows: 2 })).toEqual({
      minRows: 6,
      maxRows: 6,
    });
  });
});
