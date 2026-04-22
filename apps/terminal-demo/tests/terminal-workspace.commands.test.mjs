import test from "node:test";
import assert from "node:assert/strict";

import { createTerminalWorkspacePageCommands } from "../dist/features/terminal-workspace/renderer/commands/createTerminalWorkspacePageCommands.js";

test("page commands expose renderer intents without leaking raw draft fields or terminal bytes into UI", async () => {
  const calls = {
    createFields: [],
    inputDrafts: [],
    imports: [],
    shortcuts: [],
    toggles: 0,
    selectedSessionId: null,
  };

  const commands = createTerminalWorkspacePageCommands({
    controller: {
      createNativeSession: async () => undefined,
      selectSession: async (sessionId) => {
        calls.selectedSessionId = sessionId;
      },
      importSession: async (input) => {
        calls.imports.push(input);
      },
      restoreSavedSession: async () => undefined,
      deleteSavedSession: async () => undefined,
      refreshCatalog: async () => undefined,
      newTab: async () => undefined,
      splitFocusedPane: async () => undefined,
      saveSession: async () => undefined,
      focusPane: async () => undefined,
      focusTab: async () => undefined,
      submitInput: async () => undefined,
      sendShortcut: async (data) => {
        calls.shortcuts.push(data);
      },
    },
    setCreateField: (field, value) => {
      calls.createFields.push({ field, value });
    },
    setInputDraft: (value) => {
      calls.inputDrafts.push(value);
    },
    toggleShowAllSavedSessions: () => {
      calls.toggles += 1;
    },
    lookupDiscoveredSession: (importHandle) => {
      if (importHandle === "tmux-import-1") {
        return {
          importHandle,
          backend: "tmux",
          title: "Foreign Session",
          sourceLabel: "tmux",
          degradedSemantics: [],
        };
      }

      return null;
    },
  });

  commands.createSession.setTitle("Workspace");
  commands.createSession.setArgs('-l "-c pwd"');
  commands.input.setDraft("pwd");
  await commands.sessions.select("session-1");
  await commands.discoveredSessions.importSession("tmux-import-1");
  await commands.discoveredSessions.importSession("stale-handle");
  await commands.input.sendInterrupt();
  await commands.input.recallHistory();
  await commands.input.sendEnter();
  commands.savedSessions.toggleVisibility();

  assert.deepEqual(calls.createFields, [
    { field: "createTitleDraft", value: "Workspace" },
    { field: "createArgsDraft", value: '-l "-c pwd"' },
  ]);
  assert.deepEqual(calls.inputDrafts, ["pwd"]);
  assert.equal(calls.selectedSessionId, "session-1");
  assert.deepEqual(calls.imports, [
    {
      importHandle: "tmux-import-1",
      title: "Foreign Session",
    },
    {
      importHandle: "stale-handle",
    },
  ]);
  assert.deepEqual(calls.shortcuts, ["\u0003", "\u001b[A", "\r"]);
  assert.equal(calls.toggles, 1);
});
