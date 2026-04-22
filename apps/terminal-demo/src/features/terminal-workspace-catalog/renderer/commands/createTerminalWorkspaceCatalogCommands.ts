import type {
  TerminalDiscoveredSession,
  TerminalRuntimeWorkspaceFacade,
} from "@features/terminal-workspace-kernel/contracts";
import type { TerminalWorkspaceCatalogFormState } from "../../core/application/index.js";
import type { TerminalWorkspaceCatalogCommands } from "./TerminalWorkspaceCatalogCommands.js";

export function createTerminalWorkspaceCatalogCommands(options: {
  runtime: TerminalRuntimeWorkspaceFacade;
  form: TerminalWorkspaceCatalogFormState;
  setForm(next: Partial<TerminalWorkspaceCatalogFormState>): void;
  lookupDiscoveredSession(importHandle: string): TerminalDiscoveredSession | null;
}): TerminalWorkspaceCatalogCommands {
  return {
    setTitle: (value) => options.setForm({ title: value }),
    setProgram: (value) => options.setForm({ program: value }),
    setArgs: (value) => options.setForm({ args: value }),
    setCwd: (value) => options.setForm({ cwd: value }),
    submitCreate: () => options.runtime.commands.createNativeSession(options.form),
    selectSession: (sessionId) => options.runtime.commands.selectSession(sessionId),
    importSession: (importHandle) => options.runtime.commands.importSession(
      toImportSessionInput(importHandle, options.lookupDiscoveredSession(importHandle)),
    ),
  };
}

function toImportSessionInput(
  importHandle: string,
  discovered: TerminalDiscoveredSession | null,
) {
  if (discovered?.title) {
    return {
      importHandle,
      title: discovered.title,
    };
  }

  return {
    importHandle,
  };
}
