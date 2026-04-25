import { describe, expect, it } from "vitest";

import {
  shouldBufferTerminalDirectInput,
  shouldRefreshAfterTerminalDirectInput,
  TerminalDirectInputBuffer,
} from "./terminal-direct-input-buffer.js";

describe("TerminalDirectInputBuffer", () => {
  it("batches printable input until the scheduled flush", () => {
    const flushes: string[] = [];
    const scheduledCallbacks: Array<() => void> = [];
    const buffer = createBuffer(flushes, scheduledCallbacks);

    buffer.push("p");
    buffer.push("w");
    buffer.push("d");

    expect(flushes).toEqual([]);
    expect(scheduledCallbacks).toHaveLength(1);

    scheduledCallbacks[0]?.();

    expect(flushes).toEqual(["pwd"]);
  });

  it("flushes pending printable input before immediate terminal controls", () => {
    const flushes: string[] = [];
    const buffer = createBuffer(flushes);

    buffer.push("c");
    buffer.push("d");
    buffer.push(" ");
    buffer.push("\r");

    expect(flushes).toEqual(["cd ", "\r"]);
  });

  it("flushes pending input on dispose", () => {
    const flushes: string[] = [];
    const buffer = createBuffer(flushes);

    buffer.push("e");
    buffer.push("x");
    buffer.dispose();

    expect(flushes).toEqual(["ex"]);
  });
});

describe("terminal direct input policy", () => {
  it("buffers only printable single-byte terminal input", () => {
    expect(shouldBufferTerminalDirectInput("a")).toBe(true);
    expect(shouldBufferTerminalDirectInput(" ")).toBe(true);
    expect(shouldBufferTerminalDirectInput("\t")).toBe(false);
    expect(shouldBufferTerminalDirectInput("\r")).toBe(false);
    expect(shouldBufferTerminalDirectInput("\u007f")).toBe(false);
    expect(shouldBufferTerminalDirectInput("\u001b[A")).toBe(false);
  });

  it("refreshes the attached screen after command-ending and interrupt controls", () => {
    expect(shouldRefreshAfterTerminalDirectInput("printf ok")).toBe(false);
    expect(shouldRefreshAfterTerminalDirectInput("\r")).toBe(true);
    expect(shouldRefreshAfterTerminalDirectInput("\u0003")).toBe(true);
    expect(shouldRefreshAfterTerminalDirectInput("\u0004")).toBe(true);
    expect(shouldRefreshAfterTerminalDirectInput("echo ok\r")).toBe(true);
  });
});

function createBuffer(
  flushes: string[],
  scheduledCallbacks: Array<() => void> = [],
): TerminalDirectInputBuffer {
  return new TerminalDirectInputBuffer({
    flush: (input) => flushes.push(input),
    schedule: (callback) => {
      scheduledCallbacks.push(callback);
      return scheduledCallbacks.length as unknown as ReturnType<typeof setTimeout>;
    },
    cancel: () => undefined,
  });
}
