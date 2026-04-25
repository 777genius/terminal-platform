import { describe, expect, it } from "vitest";

import {
  isTerminalScreenSearchShortcut,
  type TerminalScreenShortcutEvent,
} from "./terminal-screen-shortcuts.js";

describe("isTerminalScreenSearchShortcut", () => {
  it("claims terminal-local search shortcuts while the screen viewport is focused", () => {
    expect(isSearchShortcut({ key: "f", ctrlKey: true })).toBe(true);
    expect(isSearchShortcut({ key: "F", metaKey: true })).toBe(true);
  });

  it("leaves composing, alternate, and broader workspace shortcuts alone", () => {
    expect(isSearchShortcut({ key: "f", ctrlKey: true, isComposing: true })).toBe(false);
    expect(isSearchShortcut({ key: "f", ctrlKey: true, altKey: true })).toBe(false);
    expect(isSearchShortcut({ key: "f", metaKey: true, shiftKey: true })).toBe(false);
    expect(isSearchShortcut({ key: "f" })).toBe(false);
    expect(isSearchShortcut({ key: "k", metaKey: true })).toBe(false);
  });
});

function isSearchShortcut(overrides: Partial<TerminalScreenShortcutEvent>): boolean {
  return isTerminalScreenSearchShortcut({
    key: "",
    altKey: false,
    ctrlKey: false,
    metaKey: false,
    ...overrides,
  });
}
