import { describe, expect, it } from "vitest";

import {
  countTerminalOutputSearchMatches,
  createTerminalOutputSearchResult,
  formatTerminalOutputSearchCount,
  resolveTerminalOutputSearchMatchIndex,
  serializeTerminalOutputLines,
} from "./terminal-output-search.js";

describe("terminal output search selectors", () => {
  it("builds case-insensitive line segments with a clamped active match", () => {
    const result = createTerminalOutputSearchResult(
      ["Ready prompt", "done READY"],
      " ready ",
      { activeMatchIndex: 99 },
    );

    expect(result.query).toBe("ready");
    expect(result.matchCount).toBe(2);
    expect(result.activeMatchIndex).toBe(1);
    expect(result.lines).toEqual([
      {
        lineIndex: 0,
        text: "Ready prompt",
        matchCount: 1,
        segments: [
          { kind: "match", value: "Ready", matchIndex: 0, active: false },
          { kind: "text", value: " prompt" },
        ],
      },
      {
        lineIndex: 1,
        text: "done READY",
        matchCount: 1,
        segments: [
          { kind: "text", value: "done " },
          { kind: "match", value: "READY", matchIndex: 1, active: true },
        ],
      },
    ]);
  });

  it("keeps blank lines renderable and ignores blank search queries", () => {
    const result = createTerminalOutputSearchResult(["", "  "], "   ");

    expect(result.matchCount).toBe(0);
    expect(result.activeMatchIndex).toBeNull();
    expect(result.lines.map((line) => line.segments)).toEqual([
      [{ kind: "text", value: " " }],
      [{ kind: "text", value: "  " }],
    ]);
  });

  it("counts non-overlapping matches across lines", () => {
    expect(countTerminalOutputSearchMatches(["aaaa", "aa"], "aa")).toBe(3);
  });

  it("formats search status for idle, empty and active states", () => {
    expect(formatTerminalOutputSearchCount("", 0, null)).toBe("Search output");
    expect(formatTerminalOutputSearchCount("missing", 0, null)).toBe("0 matches");
    expect(formatTerminalOutputSearchCount("ready", 3, 1)).toBe("2 of 3");
  });

  it("resolves active match indexes safely", () => {
    expect(resolveTerminalOutputSearchMatchIndex(null, 3)).toBe(0);
    expect(resolveTerminalOutputSearchMatchIndex(-1, 3)).toBe(0);
    expect(resolveTerminalOutputSearchMatchIndex(5, 3)).toBe(2);
    expect(resolveTerminalOutputSearchMatchIndex(0, 0)).toBeNull();
  });

  it("serializes visible terminal output without trailing whitespace noise", () => {
    expect(serializeTerminalOutputLines(["one", "two  ", ""])).toBe("one\ntwo");
  });
});
