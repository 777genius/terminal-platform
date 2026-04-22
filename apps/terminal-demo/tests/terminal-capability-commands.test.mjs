import test from "node:test";
import assert from "node:assert/strict";

import { createTerminalWorkspaceCatalogCommands } from "../dist/features/terminal-workspace-catalog/renderer/commands/createTerminalWorkspaceCatalogCommands.js";
import { createTerminalInputComposerCommands } from "../dist/features/terminal-input-composer/renderer/commands/createTerminalInputComposerCommands.js";
import { createTerminalSavedSessionsCommands } from "../dist/features/terminal-saved-sessions/renderer/commands/createTerminalSavedSessionsCommands.js";
import { createTerminalActiveSessionCommands } from "../dist/features/terminal-active-session/renderer/commands/createTerminalActiveSessionCommands.js";

test("capability command facades expose domain intents without leaking transport or store details", async () => {
  const calls = {
    createInputs: [],
    selectedSessionId: null,
    imports: [],
    inputDrafts: [],
    submittedInputs: [],
    shortcuts: [],
    restores: [],
    deletes: [],
    toggles: 0,
    refreshes: 0,
    newTabs: 0,
    splits: [],
    saves: 0,
    focusedPanes: [],
    focusedTabs: [],
  };

  const runtime = {
    commands: {
      createNativeSession: async (input) => {
        calls.createInputs.push(input);
      },
      selectSession: async (sessionId) => {
        calls.selectedSessionId = sessionId;
      },
      importSession: async (input) => {
        calls.imports.push(input);
      },
      restoreSavedSession: async (sessionId) => {
        calls.restores.push(sessionId);
      },
      deleteSavedSession: async (sessionId) => {
        calls.deletes.push(sessionId);
      },
      refreshCatalog: async () => {
        calls.refreshes += 1;
      },
      newTab: async () => {
        calls.newTabs += 1;
      },
      splitFocusedPane: async (direction) => {
        calls.splits.push(direction);
      },
      saveSession: async () => {
        calls.saves += 1;
      },
      focusPane: async (paneId) => {
        calls.focusedPanes.push(paneId);
      },
      focusTab: async (tabId) => {
        calls.focusedTabs.push(tabId);
      },
      submitInput: async (input) => {
        calls.submittedInputs.push(input);
        return true;
      },
      sendShortcut: async (data) => {
        calls.shortcuts.push(data);
      },
    },
  };

  let form = {
    title: "Workspace",
    program: "",
    args: "",
    cwd: "",
  };
  const catalogCommands = createTerminalWorkspaceCatalogCommands({
    runtime,
    form,
    setForm(next) {
      Object.assign(form, next);
    },
    lookupDiscoveredSession(importHandle) {
      return importHandle === "tmux-import-1"
        ? {
            importHandle,
            backend: "tmux",
            title: "Foreign Session",
            sourceLabel: "tmux",
            degradedSemantics: [],
          }
        : null;
    },
  });

  let draft = "pwd";
  const inputCommands = createTerminalInputComposerCommands({
    runtime,
    draft,
    setDraft(value) {
      draft = value;
      calls.inputDrafts.push(value);
    },
  });

  const savedCommands = createTerminalSavedSessionsCommands({
    runtime,
    toggleVisibility() {
      calls.toggles += 1;
    },
  });
  const activeCommands = createTerminalActiveSessionCommands(runtime);

  catalogCommands.setTitle("Workspace Alpha");
  catalogCommands.setArgs('-l "-c pwd"');
  await catalogCommands.submitCreate();
  await catalogCommands.selectSession("session-1");
  await catalogCommands.importSession("tmux-import-1");
  await catalogCommands.importSession("stale-import");

  inputCommands.setDraft("ls");
  await inputCommands.submit();
  await inputCommands.sendInterrupt();
  await inputCommands.recallHistory();
  await inputCommands.sendEnter();

  await savedCommands.restore("saved-1");
  await savedCommands.delete("saved-1");
  savedCommands.toggleVisibility();

  await activeCommands.refreshCatalog();
  await activeCommands.newTab();
  await activeCommands.splitHorizontal();
  await activeCommands.splitVertical();
  await activeCommands.saveSession();
  await activeCommands.focusPane("pane-1");
  await activeCommands.focusTab("tab-1");

  assert.deepEqual(calls.createInputs, [
    {
      title: "Workspace Alpha",
      program: "",
      args: '-l "-c pwd"',
      cwd: "",
    },
  ]);
  assert.equal(calls.selectedSessionId, "session-1");
  assert.deepEqual(calls.imports, [
    {
      importHandle: "tmux-import-1",
      title: "Foreign Session",
    },
    {
      importHandle: "stale-import",
    },
  ]);
  assert.deepEqual(calls.inputDrafts, ["ls", ""]);
  assert.deepEqual(calls.submittedInputs, ["pwd"]);
  assert.deepEqual(calls.shortcuts, ["\u0003", "\u001b[A", "\r"]);
  assert.deepEqual(calls.restores, ["saved-1"]);
  assert.deepEqual(calls.deletes, ["saved-1"]);
  assert.equal(calls.toggles, 1);
  assert.equal(calls.refreshes, 1);
  assert.equal(calls.newTabs, 1);
  assert.deepEqual(calls.splits, ["horizontal", "vertical"]);
  assert.equal(calls.saves, 1);
  assert.deepEqual(calls.focusedPanes, ["pane-1"]);
  assert.deepEqual(calls.focusedTabs, ["tab-1"]);
});
