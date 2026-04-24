import { describe, expect, it } from "vitest";

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
});
