import { describe, expect, it } from "vitest";

import { terminalElementStyles } from "./styles.js";

describe("workspace elements styles public subpath", () => {
  it("exposes shared element styles without requiring a deep import", () => {
    expect(String(terminalElementStyles)).toContain("--tp-terminal-color-bg");
    expect(String(terminalElementStyles)).toContain("box-sizing: border-box");
  });
});
