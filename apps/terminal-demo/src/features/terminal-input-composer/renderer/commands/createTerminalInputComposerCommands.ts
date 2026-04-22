import type { TerminalRuntimeWorkspaceFacade } from "@features/terminal-workspace-kernel/contracts";
import type { TerminalInputComposerCommands } from "./TerminalInputComposerCommands.js";

export function createTerminalInputComposerCommands(options: {
  runtime: TerminalRuntimeWorkspaceFacade;
  draft: string;
  setDraft(value: string): void;
}): TerminalInputComposerCommands {
  return {
    setDraft: (value) => options.setDraft(value),
    submit: async () => {
      const submitted = await options.runtime.commands.submitInput(options.draft);
      if (submitted) {
        options.setDraft("");
      }
    },
    sendInterrupt: () => options.runtime.commands.sendShortcut("\u0003"),
    recallHistory: () => options.runtime.commands.sendShortcut("\u001b[A"),
    sendEnter: () => options.runtime.commands.sendShortcut("\r"),
  };
}
