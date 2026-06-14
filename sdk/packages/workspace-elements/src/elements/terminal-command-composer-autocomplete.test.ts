import { describe, expect, it } from "vitest";

import {
  resolveTerminalCommandComposerAutocomplete,
  type TerminalCommandComposerAutocompleteOptions,
} from "./terminal-command-composer-autocomplete.js";

describe("terminal command composer autocomplete", () => {
  it("resolves a ghost suffix when a focused single-line draft prefixes a suggestion", () => {
    expect(resolveAutocomplete("pnpm t", "pnpm typecheck")).toEqual({
      draft: "pnpm t",
      suffix: "ypecheck",
      suggestion: "pnpm typecheck",
    });
  });

  it("does not mutate exact matches, empty drafts, multiline input, or unfocused input", () => {
    expect(resolveAutocomplete("pnpm test", "pnpm test")).toBeNull();
    expect(resolveAutocomplete("p", "pnpm test")).toBeNull();
    expect(resolveAutocomplete("pnpm\n", "pnpm test")).toBeNull();
    expect(
      resolveAutocomplete("pnpm", "pnpm test", { inputFocused: false }),
    ).toBeNull();
  });

  it("only suggests when the caret is at the end of the draft", () => {
    expect(
      resolveTerminalCommandComposerAutocomplete(
        {
          value: "pnpm t",
          selectionStart: 2,
          selectionEnd: 2,
        },
        createOptions("pnpm typecheck"),
      ),
    ).toBeNull();

    expect(
      resolveTerminalCommandComposerAutocomplete(
        {
          value: "pnpm t",
          selectionStart: 2,
          selectionEnd: 6,
        },
        createOptions("pnpm typecheck"),
      ),
    ).toBeNull();
  });

  it("guards disabled, composing, unrelated, and overly long drafts", () => {
    expect(
      resolveAutocomplete("pnpm t", "pnpm typecheck", { canWriteInput: false }),
    ).toBeNull();
    expect(
      resolveAutocomplete("pnpm t", "pnpm typecheck", { isComposing: true }),
    ).toBeNull();
    expect(resolveAutocomplete("git", "pnpm test")).toBeNull();
    expect(resolveAutocomplete("pnpm t", "pnpm\ntest")).toBeNull();
    expect(
      resolveAutocomplete("x".repeat(161), `${"x".repeat(161)}y`),
    ).toBeNull();
  });
});

function resolveAutocomplete(
  draft: string,
  suggestion: string | null,
  optionOverrides: Partial<TerminalCommandComposerAutocompleteOptions> = {},
) {
  return resolveTerminalCommandComposerAutocomplete(
    {
      value: draft,
      selectionStart: draft.length,
      selectionEnd: draft.length,
    },
    createOptions(suggestion, optionOverrides),
  );
}

function createOptions(
  suggestion: string | null,
  overrides: Partial<TerminalCommandComposerAutocompleteOptions> = {},
): TerminalCommandComposerAutocompleteOptions {
  return {
    canWriteInput: true,
    inputFocused: true,
    suggestion,
    ...overrides,
  };
}
