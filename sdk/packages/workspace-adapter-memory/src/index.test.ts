import { describe, expect, it } from "vitest";

import type { PaneTreeNode } from "@terminal-platform/runtime-types";

import {
  createDefaultMemoryWorkspaceFixture,
  createMemoryWorkspaceTransport,
} from "./index.js";

describe("createMemoryWorkspaceTransport command projection", () => {
  it("projects sent input into the focused screen snapshot", async () => {
    const transport = createMemoryWorkspaceTransport();
    const session = (await transport.listSessions())[0]!;
    const attached = await transport.attachSession(session.session_id);
    const paneId = attached.focused_screen!.pane_id;
    const initialSequence = attached.focused_screen!.sequence;

    const result = await transport.dispatchMuxCommand(session.session_id, {
      kind: "send_input",
      pane_id: paneId,
      data: 'printf "memory-ok\\n"\n',
    });

    const updated = await transport.attachSession(session.session_id);
    const screen = await transport.getScreenSnapshot(session.session_id, paneId);
    const delta = await transport.getScreenDelta(session.session_id, paneId, initialSequence);

    expect(result.changed).toBe(true);
    expect(updated.focused_screen?.sequence).toBe(initialSequence + 1n);
    expect(screen.surface.lines.map((line) => line.text)).toContain('$ printf "memory-ok\\n"');
    expect(screen.surface.cursor).toEqual({
      row: screen.surface.lines.length - 1,
      col: '$ printf "memory-ok\\n"'.length,
    });
    expect(delta.from_sequence).toBe(initialSequence);
    expect(delta.to_sequence).toBe(initialSequence + 1n);
    expect(delta.full_replace?.lines.map((line) => line.text)).toContain('$ printf "memory-ok\\n"');

    await transport.close();
  });

  it("renders shortcut and paste input explicitly", async () => {
    const transport = createMemoryWorkspaceTransport();
    const session = (await transport.listSessions())[0]!;
    const attached = await transport.attachSession(session.session_id);
    const paneId = attached.focused_screen!.pane_id;

    await transport.dispatchMuxCommand(session.session_id, {
      kind: "send_input",
      pane_id: paneId,
      data: "\u0003",
    });
    await transport.dispatchMuxCommand(session.session_id, {
      kind: "send_paste",
      pane_id: paneId,
      data: "first\nsecond\n",
    });

    const screen = await transport.getScreenSnapshot(session.session_id, paneId);

    expect(screen.surface.lines.map((line) => line.text).slice(-3)).toEqual([
      "^C",
      "paste first",
      "second",
    ]);

    await transport.close();
  });

  it("projects pane resize commands into screen snapshots and deltas", async () => {
    const transport = createMemoryWorkspaceTransport();
    const session = (await transport.listSessions())[0]!;
    const attached = await transport.attachSession(session.session_id);
    const paneId = attached.focused_screen!.pane_id;
    const initialSequence = attached.focused_screen!.sequence;

    await transport.dispatchMuxCommand(session.session_id, {
      kind: "resize_pane",
      pane_id: paneId,
      rows: 12,
      cols: 96,
    });

    const updated = await transport.attachSession(session.session_id);
    const delta = await transport.getScreenDelta(session.session_id, paneId, initialSequence);

    expect(updated.focused_screen?.rows).toBe(12);
    expect(updated.focused_screen?.cols).toBe(96);
    expect(updated.focused_screen?.sequence).toBe(initialSequence + 1n);
    expect(delta.rows).toBe(12);
    expect(delta.cols).toBe(96);
    expect(delta.full_replace?.cursor?.row).toBeLessThan(12);

    await transport.close();
  });

  it("saves terminal layouts as immutable snapshots", async () => {
    const transport = createMemoryWorkspaceTransport();
    const session = (await transport.listSessions())[0]!;
    const attached = await transport.attachSession(session.session_id);
    const paneId = attached.focused_screen!.pane_id;
    const initialSaved = await transport.listSavedSessions();

    await transport.dispatchMuxCommand(session.session_id, {
      kind: "send_input",
      pane_id: paneId,
      data: "printf saved-layout\n",
    });
    await transport.dispatchMuxCommand(session.session_id, { kind: "save_session" });

    const afterSave = await transport.listSavedSessions();
    const savedRecord = await transport.getSavedSession(afterSave[0]!.session_id);

    await transport.dispatchMuxCommand(session.session_id, {
      kind: "send_input",
      pane_id: paneId,
      data: "printf after-save\n",
    });
    const savedRecordAfterLiveMutation = await transport.getSavedSession(afterSave[0]!.session_id);

    expect(afterSave).toHaveLength(initialSaved.length + 1);
    expect(afterSave[0]!.session_id).toBe(`${session.session_id}-saved-${initialSaved.length + 1}`);
    expect(afterSave[0]!.pane_count).toBe(1);
    expect(savedRecord.screens[0]?.surface.lines.map((line) => line.text)).toContain("$ printf saved-layout");
    expect(savedRecordAfterLiveMutation.screens[0]?.surface.lines.map((line) => line.text)).not.toContain(
      "$ printf after-save",
    );

    await transport.close();
  });

  it("projects tab lifecycle, split, focus, and rename commands into topology snapshots", async () => {
    const transport = createMemoryWorkspaceTransport();
    const session = (await transport.listSessions())[0]!;
    const initial = await transport.attachSession(session.session_id);
    const initialPaneId = initial.focused_screen!.pane_id;
    const initialTabId = initial.topology.tabs[0]!.tab_id;

    await transport.dispatchMuxCommand(session.session_id, {
      kind: "new_tab",
      title: "logs",
    });
    const afterNewTab = await transport.attachSession(session.session_id);
    const createdTab = afterNewTab.topology.tabs.find((tab) => tab.tab_id !== initialTabId)!;
    expect(afterNewTab.topology.tabs).toHaveLength(2);
    expect(afterNewTab.topology.focused_tab).toBe(createdTab.tab_id);
    expect(afterNewTab.focused_screen?.pane_id).toBe(createdTab.focused_pane);

    await transport.dispatchMuxCommand(session.session_id, {
      kind: "split_pane",
      pane_id: createdTab.focused_pane!,
      direction: "horizontal",
    });
    const afterSplit = await transport.attachSession(session.session_id);
    const splitTab = afterSplit.topology.tabs.find((tab) => tab.tab_id === createdTab.tab_id)!;
    expect(splitTab.root.kind).toBe("split");
    expect(splitTab.focused_pane).not.toBe(createdTab.focused_pane);
    expect(afterSplit.focused_screen?.pane_id).toBe(splitTab.focused_pane);

    await transport.dispatchMuxCommand(session.session_id, {
      kind: "focus_pane",
      pane_id: initialPaneId,
    });
    const afterFocusPane = await transport.attachSession(session.session_id);
    expect(afterFocusPane.topology.focused_tab).toBe(initialTabId);
    expect(afterFocusPane.focused_screen?.pane_id).toBe(initialPaneId);

    await transport.dispatchMuxCommand(session.session_id, {
      kind: "focus_tab",
      tab_id: createdTab.tab_id,
    });
    await transport.dispatchMuxCommand(session.session_id, {
      kind: "rename_tab",
      tab_id: createdTab.tab_id,
      title: "renamed logs",
    });
    const afterRename = await transport.getTopologySnapshot(session.session_id);
    expect(afterRename.focused_tab).toBe(createdTab.tab_id);
    expect(afterRename.tabs.find((tab) => tab.tab_id === createdTab.tab_id)?.title).toBe("renamed logs");

    const splitPaneIds = collectPaneIds(splitTab.root);
    const paneToClose = splitPaneIds.find((paneId) => paneId !== splitTab.focused_pane)!;
    await transport.dispatchMuxCommand(session.session_id, {
      kind: "close_pane",
      pane_id: paneToClose,
    });
    const afterClosePane = await transport.attachSession(session.session_id);
    const afterClosePaneTab = afterClosePane.topology.tabs.find((tab) => tab.tab_id === createdTab.tab_id)!;
    expect(collectPaneIds(afterClosePaneTab.root)).not.toContain(paneToClose);
    expect(afterClosePaneTab.focused_pane).toBe(splitTab.focused_pane);
    await expect(transport.getScreenSnapshot(session.session_id, paneToClose)).rejects.toMatchObject({
      code: "session_not_found",
    });

    await transport.dispatchMuxCommand(session.session_id, {
      kind: "close_tab",
      tab_id: createdTab.tab_id,
    });
    const afterCloseTab = await transport.attachSession(session.session_id);
    expect(afterCloseTab.topology.tabs).toHaveLength(1);
    expect(afterCloseTab.topology.focused_tab).toBe(initialTabId);
    expect(afterCloseTab.focused_screen?.pane_id).toBe(initialPaneId);
    await expect(transport.dispatchMuxCommand(session.session_id, {
      kind: "close_tab",
      tab_id: initialTabId,
    })).rejects.toMatchObject({ code: "unsupported_capability" });
    await expect(transport.dispatchMuxCommand(session.session_id, {
      kind: "close_pane",
      pane_id: initialPaneId,
    })).rejects.toMatchObject({ code: "unsupported_capability" });

    await transport.close();
  });

  it("preserves sibling branches when closing nested panes", async () => {
    const transport = createMemoryWorkspaceTransport();
    const session = (await transport.listSessions())[0]!;
    const initial = await transport.attachSession(session.session_id);
    const originalPaneId = initial.focused_screen!.pane_id;

    await transport.dispatchMuxCommand(session.session_id, {
      kind: "split_pane",
      pane_id: originalPaneId,
      direction: "horizontal",
    });
    const afterFirstSplit = await transport.attachSession(session.session_id);
    const paneToNest = afterFirstSplit.topology.tabs[0]!.focused_pane!;
    await transport.dispatchMuxCommand(session.session_id, {
      kind: "split_pane",
      pane_id: paneToNest,
      direction: "vertical",
    });

    const afterNestedSplit = await transport.attachSession(session.session_id);
    const nestedTab = afterNestedSplit.topology.tabs[0]!;
    const focusedPaneId = nestedTab.focused_pane!;
    const nestedPaneToClose = collectPaneIds(nestedTab.root).find(
      (paneId) => paneId !== originalPaneId && paneId !== focusedPaneId,
    )!;
    await transport.dispatchMuxCommand(session.session_id, {
      kind: "close_pane",
      pane_id: nestedPaneToClose,
    });

    const afterClose = await transport.attachSession(session.session_id);
    const remainingPaneIds = collectPaneIds(afterClose.topology.tabs[0]!.root);
    expect(remainingPaneIds).toHaveLength(2);
    expect(remainingPaneIds).toContain(originalPaneId);
    expect(remainingPaneIds).toContain(focusedPaneId);

    await transport.close();
  });

  it("keeps saved layout ids monotonic for custom fixtures", async () => {
    const fixture = createDefaultMemoryWorkspaceFixture();
    const seedRecord = Object.values(fixture.savedSessionRecords)[0]!;
    const seededId = `${fixture.sessions[0]!.session_id}-saved-9`;
    fixture.savedSessions = [
      {
        ...fixture.savedSessions[0]!,
        session_id: seededId,
      },
    ];
    fixture.savedSessionRecords = {
      [seededId]: {
        ...seedRecord,
        session_id: seededId,
      },
    };

    const transport = createMemoryWorkspaceTransport({ fixture });
    const session = (await transport.listSessions())[0]!;

    await transport.dispatchMuxCommand(session.session_id, { kind: "save_session" });

    const saved = await transport.listSavedSessions();
    expect(saved[0]!.session_id).toBe(`${session.session_id}-saved-10`);

    await transport.close();
  });

  it("restores saved memory layouts with their saved screen contents", async () => {
    const transport = createMemoryWorkspaceTransport();
    const session = (await transport.listSessions())[0]!;
    const attached = await transport.attachSession(session.session_id);
    const paneId = attached.focused_screen!.pane_id;

    await transport.dispatchMuxCommand(session.session_id, {
      kind: "send_input",
      pane_id: paneId,
      data: "printf restore-me\n",
    });
    await transport.dispatchMuxCommand(session.session_id, { kind: "save_session" });

    const saved = (await transport.listSavedSessions())[0]!;
    const restored = await transport.restoreSavedSession(saved.session_id);
    const restoredAttached = await transport.attachSession(restored.session.session_id);

    expect(restored.saved_session_id).toBe(saved.session_id);
    expect(restoredAttached.focused_screen?.surface.lines.map((line) => line.text)).toContain(
      "$ printf restore-me",
    );

    await transport.close();
  });

  it("rejects incompatible saved memory layouts without creating sessions", async () => {
    const fixture = createDefaultMemoryWorkspaceFixture();
    const saved = fixture.savedSessions[0]!;
    const record = fixture.savedSessionRecords[saved.session_id]!;
    const compatibility = {
      can_restore: false,
      status: "protocol_minor_ahead" as const,
    };
    saved.compatibility = compatibility;
    record.compatibility = compatibility;

    const transport = createMemoryWorkspaceTransport({ fixture });
    const sessionCountBefore = (await transport.listSessions()).length;

    await expect(transport.restoreSavedSession(saved.session_id)).rejects.toMatchObject({
      code: "unsupported_capability",
      recoverable: false,
    });
    expect(await transport.listSessions()).toHaveLength(sessionCountBefore);

    await transport.close();
  });

  it("rejects unknown saved layout deletion", async () => {
    const transport = createMemoryWorkspaceTransport();
    const savedCountBefore = (await transport.listSavedSessions()).length;

    await expect(transport.deleteSavedSession("missing-saved")).rejects.toMatchObject({
      code: "session_not_found",
      recoverable: false,
    });
    expect(await transport.listSavedSessions()).toHaveLength(savedCountBefore);

    await transport.close();
  });

  it("rejects invalid saved layout prune limits", async () => {
    const transport = createMemoryWorkspaceTransport();
    const savedCountBefore = (await transport.listSavedSessions()).length;

    await expect(transport.pruneSavedSessions(-1)).rejects.toMatchObject({
      code: "protocol_error",
      recoverable: false,
    });
    await expect(transport.pruneSavedSessions(Number.NaN)).rejects.toMatchObject({
      code: "protocol_error",
      recoverable: false,
    });
    expect(await transport.listSavedSessions()).toHaveLength(savedCountBefore);

    await transport.close();
  });
});

function collectPaneIds(node: PaneTreeNode): string[] {
  if (node.kind === "leaf") {
    return [node.pane_id];
  }

  return [...collectPaneIds(node.first), ...collectPaneIds(node.second)];
}
