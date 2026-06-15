import { describe, expect, it } from "vitest";

import { TerminalCommandComposerElement } from "./terminal-command-composer-element.js";
import { TerminalCommandDockElement } from "./terminal-command-dock-element.js";

describe("terminal command dock autocomplete styles", () => {
  it("keeps the ghost suggestion aligned inline with the command draft", () => {
    const styles = String(TerminalCommandDockElement.styles);

    expect(styles).toContain(".autocomplete-ghost");
    expect(styles).toContain("inline-size: 100%");
    expect(styles).toContain("justify-self: stretch");
    expect(styles).toContain("text-align: left");
    expect(styles).toContain(".autocomplete-ghost-prefix");
    expect(styles).toContain("display: none");
    expect(styles).toContain(".autocomplete-ghost-suffix");
    expect(styles).toContain("margin-inline-start");
    expect(styles).toContain("--tp-command-autocomplete-prefix-width");
    expect(styles).toContain("white-space: pre");
  });

  it("renders the autocomplete ghost with a deterministic prefix width", () => {
    const renderSource = String(TerminalCommandComposerElement.prototype.render);

    expect(renderSource).toContain(
      'data-testid="tp-command-autocomplete-ghost"'
    );
    expect(renderSource).toContain("data-prefix-length");
    expect(renderSource).toContain(
      "--tp-command-autocomplete-prefix-width: ${autocomplete.draft.length}ch;"
    );
    expect(renderSource).toContain(
      '><span class="autocomplete-ghost-prefix"'
    );
    expect(renderSource).not.toContain(
      '>\n            <span class="autocomplete-ghost-prefix"'
    );
  });
});
