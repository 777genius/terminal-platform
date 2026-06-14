import { describe, expect, it } from "vitest";

import { TERMINAL_COMMAND_COMPOSER_EVENTS } from "./terminal-command-composer-events.js";

describe("terminal command composer events", () => {
  it("exposes stable event names for framework adapters", () => {
    expect(TERMINAL_COMMAND_COMPOSER_EVENTS).toEqual({
      autocompleteAccept: "tp-terminal-command-autocomplete-accept",
      autocompleteDismiss: "tp-terminal-command-autocomplete-dismiss",
      draftChange: "tp-terminal-command-draft-change",
      historyNavigate: "tp-terminal-command-history-navigate",
      paste: "tp-terminal-command-paste",
      shortcut: "tp-terminal-command-shortcut",
      submit: "tp-terminal-command-submit",
    });
  });
});
