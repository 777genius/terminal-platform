import { describe, expect, it } from "vitest";

import { terminalInputForKeyboardEvent, type TerminalKeyboardInputEvent } from "./terminal-keyboard-input.js";

describe("terminalInputForKeyboardEvent", () => {
  it("maps printable keys and terminal navigation keys to input bytes", () => {
    expect(inputFor({ key: "a" })).toBe("a");
    expect(inputFor({ key: "Enter" })).toBe("\r");
    expect(inputFor({ key: "Tab" })).toBe("\t");
    expect(inputFor({ key: "Backspace" })).toBe("\u007f");
    expect(inputFor({ key: "ArrowUp" })).toBe("\u001b[A");
    expect(inputFor({ key: "ArrowDown" })).toBe("\u001b[B");
    expect(inputFor({ key: "Delete" })).toBe("\u001b[3~");
  });

  it("maps focused terminal control chords without stealing browser meta shortcuts", () => {
    expect(inputFor({ key: "c", ctrlKey: true })).toBe("\u0003");
    expect(inputFor({ key: "l", ctrlKey: true })).toBe("\u000c");
    expect(inputFor({ key: "v", ctrlKey: true })).toBeNull();
    expect(inputFor({ key: "l", metaKey: true })).toBeNull();
    expect(inputFor({ key: "a", altKey: true })).toBeNull();
  });
});

function inputFor(overrides: Partial<TerminalKeyboardInputEvent>): string | null {
  return terminalInputForKeyboardEvent({
    key: "",
    altKey: false,
    ctrlKey: false,
    metaKey: false,
    ...overrides,
  });
}
