import { describe, expect, it } from "vitest";

import { projectTerminalHistoryText } from "./terminal-history-text-projector.js";

describe("projectTerminalHistoryText", () => {
  it("replays shell backspace redraws instead of duplicating the first command character", () => {
    const projection = projectTerminalHistoryText("shell % p\bprint 'fdfd'\r\nfdfd\r\nshell % ");

    expect(projection.lines.map((line) => line.text)).toEqual([
      "shell % print 'fdfd'",
      "fdfd",
      "shell %",
    ]);
  });

  it("replays carriage-return and erase redraws across soft-wrapped command rows", () => {
    const prompt = "(base) belief@MacBook-Pro-belief agent-teams-terminal-ui-e2e-v031 % ";
    const command =
      "printf 'WRAP_OK_V031_%s\\n' 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ'";
    const redraw = [
      `${prompt}p\bprintf 'WRAP `,
      "\r\x1b[K_",
      "\r_OK_V031_%s\\n' 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789AB ",
      "\r\x1b[KC",
      "\rCDEFGHIJKLMNOPQRSTUVWXYZ'",
      "\r\n",
    ].join("");
    const projection = projectTerminalHistoryText(redraw, {
      columns: 80,
      rows: 24,
    });
    const renderedCommand = joinSoftWrappedLines(
      projection.lines.map((line) => ({
        text: line.text,
        wrapped: line.wrapped === true,
      }))
    );

    expect(renderedCommand).toContain(`${prompt}${command}`);
    expect(renderedCommand).not.toContain("pprintf");
    expect(renderedCommand).not.toContain("__OK");
    expect(projection.lines.some((line) => line.wrapped === true)).toBe(true);
  });

  it("preserves ANSI foreground colors in durable history spans", () => {
    const projection = projectTerminalHistoryText(
      "plain\r\n\x1b[31mRED_V031\x1b[0m\r\n\x1b[38;2;1;2;3mRGB\x1b[0m\r\n"
    );

    expect(projection.lines[1]).toMatchObject({
      text: "RED_V031",
      spans: [
        {
          text: "RED_V031",
          style: { foreground: { kind: "named", name: "red" } },
        },
      ],
    });
    expect(projection.lines[2]).toMatchObject({
      text: "RGB",
      spans: [
        {
          text: "RGB",
          style: { foreground: { kind: "rgb", r: 1, g: 2, b: 3 } },
        },
      ],
    });
  });

  it("applies progress rewrites, cursor movement, and erase-in-line", () => {
    const projection = projectTerminalHistoryText(
      "progress 10%\rprogress 20%\x1b[K\r\nabcde\x1b[2DXY\x1b[K\r\n"
    );

    expect(projection.lines.map((line) => line.text)).toEqual(["progress 20%", "abcXY"]);
  });

  it("consumes OSC and private-mode controls without leaking terminal bytes", () => {
    const projection = projectTerminalHistoryText(
      "\x1b]0;secret title\x07\x1b[?2004hready\x1b[?2004l\r\n"
    );

    expect(projection.lines.map((line) => line.text)).toEqual(["ready"]);
    expect(projection.startsWithLineBreak).toBe(false);
    expect(projection.endsWithLineBreak).toBe(true);
  });
});

function joinSoftWrappedLines(lines: readonly { text: string; wrapped: boolean }[]): string {
  let result = "";
  for (const line of lines) {
    result += line.text;
    if (!line.wrapped) result += "\n";
  }
  return result;
}
