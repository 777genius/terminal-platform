import { describe, expect, it } from "vitest";

import { readClipboardText, writeClipboardText } from "./terminal-clipboard.js";

describe("terminal clipboard helpers", () => {
  it("reads text through the injected clipboard adapter", async () => {
    await expect(readClipboardText({
      clipboard: {
        readText: async () => "printf ok",
      },
    })).resolves.toBe("printf ok");
  });

  it("writes text through the injected clipboard adapter", async () => {
    let copiedText = "";

    await writeClipboardText("visible output", {
      clipboard: {
        writeText: async (value) => {
          copiedText = value;
        },
      },
    });

    expect(copiedText).toBe("visible output");
  });

  it("fails fast when clipboard read is unavailable", async () => {
    await expect(readClipboardText({ clipboard: null }))
      .rejects.toThrow("Clipboard read is unavailable");
  });

  it("fails fast when clipboard write is unavailable", async () => {
    await expect(writeClipboardText("output", { clipboard: null }))
      .rejects.toThrow("Clipboard write is unavailable");
  });

  it("times out hung clipboard reads", async () => {
    await expect(readClipboardText({
      clipboard: {
        readText: () => new Promise<string>(() => undefined),
      },
      timeoutMs: 1,
    })).rejects.toThrow("Clipboard read timed out");
  });

  it("times out hung clipboard writes", async () => {
    await expect(writeClipboardText("output", {
      clipboard: {
        writeText: () => new Promise<void>(() => undefined),
      },
      timeoutMs: 1,
    })).rejects.toThrow("Clipboard write timed out");
  });
});
