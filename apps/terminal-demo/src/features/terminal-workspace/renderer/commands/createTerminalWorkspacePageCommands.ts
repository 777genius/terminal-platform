import type { TerminalDiscoveredSession, TerminalImportSessionInput } from "../../contracts/terminal-workspace-contracts.js";
import type { TerminalWorkspaceController } from "../../core/application/index.js";
import type { TerminalWorkspacePageCommands } from "./TerminalWorkspacePageCommands.js";

type CreateFieldName =
  | "createTitleDraft"
  | "createProgramDraft"
  | "createArgsDraft"
  | "createCwdDraft";

interface TerminalWorkspacePageCommandsOptions {
  controller: Pick<
    TerminalWorkspaceController,
    | "createNativeSession"
    | "selectSession"
    | "importSession"
    | "restoreSavedSession"
    | "deleteSavedSession"
    | "refreshCatalog"
    | "newTab"
    | "splitFocusedPane"
    | "saveSession"
    | "focusPane"
    | "focusTab"
    | "submitInput"
    | "sendShortcut"
  >;
  setCreateField(field: CreateFieldName, value: string): void;
  setInputDraft(value: string): void;
  toggleShowAllSavedSessions(): void;
  lookupDiscoveredSession(importHandle: string): TerminalDiscoveredSession | null;
}

export function createTerminalWorkspacePageCommands(
  options: TerminalWorkspacePageCommandsOptions,
): TerminalWorkspacePageCommands {
  return {
    createSession: {
      setTitle: (value) => {
        options.setCreateField("createTitleDraft", value);
      },
      setProgram: (value) => {
        options.setCreateField("createProgramDraft", value);
      },
      setArgs: (value) => {
        options.setCreateField("createArgsDraft", value);
      },
      setCwd: (value) => {
        options.setCreateField("createCwdDraft", value);
      },
      submit: () => options.controller.createNativeSession(),
    },
    sessions: {
      select: (sessionId) => options.controller.selectSession(sessionId),
      refreshCatalog: () => options.controller.refreshCatalog(),
    },
    discoveredSessions: {
      importSession: (importHandle) => options.controller.importSession(
        toImportSessionInput(importHandle, options.lookupDiscoveredSession(importHandle)),
      ),
    },
    savedSessions: {
      restore: (sessionId) => options.controller.restoreSavedSession(sessionId),
      delete: (sessionId) => options.controller.deleteSavedSession(sessionId),
      toggleVisibility: () => {
        options.toggleShowAllSavedSessions();
      },
    },
    topology: {
      newTab: () => options.controller.newTab(),
      splitHorizontal: () => options.controller.splitFocusedPane("horizontal"),
      splitVertical: () => options.controller.splitFocusedPane("vertical"),
      saveSession: () => options.controller.saveSession(),
      focusPane: (paneId) => options.controller.focusPane(paneId),
      focusTab: (tabId) => options.controller.focusTab(tabId),
    },
    input: {
      setDraft: (value) => {
        options.setInputDraft(value);
      },
      submit: () => options.controller.submitInput(),
      sendInterrupt: () => options.controller.sendShortcut("\u0003"),
      recallHistory: () => options.controller.sendShortcut("\u001b[A"),
      sendEnter: () => options.controller.sendShortcut("\r"),
    },
  };
}

function toImportSessionInput(
  importHandle: string,
  discovered: TerminalDiscoveredSession | null,
): TerminalImportSessionInput {
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
