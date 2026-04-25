import { describe, expect, it } from "vitest";

import type { SavedSessionCompatibilityStatus, SavedSessionSummary } from "@terminal-platform/runtime-types";
import { createInitialWorkspaceSnapshot, type WorkspaceSnapshot } from "@terminal-platform/workspace-core";

import {
  findRestorableSavedSession,
  hasSavedSession,
  resolveTerminalSavedSessionsControlState,
  type TerminalSavedSessionsControlOptions,
} from "./terminal-saved-sessions-controls.js";

describe("terminal saved sessions controls", () => {
  it("paginates saved sessions and exposes hidden maintenance controls", () => {
    const snapshot = createWorkspaceSnapshot(Array.from({ length: 6 }, (_, index) => createSavedSession(index + 1)));

    const controls = resolveTerminalSavedSessionsControlState(snapshot, createOptions());

    expect(controls.savedSessionCount).toBe(6);
    expect(controls.visibleCount).toBe(4);
    expect(controls.hiddenCount).toBe(2);
    expect(controls.canShowMore).toBe(true);
    expect(controls.canCollapse).toBe(false);
    expect(controls.canPruneHidden).toBe(true);
    expect(controls.pruneKeepLatest).toBe(4);
    expect(controls.items.map((item) => item.session.session_id)).toEqual([
      "saved-1",
      "saved-2",
      "saved-3",
      "saved-4",
    ]);
  });

  it("enables collapse after expansion and hides prune when all entries are visible", () => {
    const snapshot = createWorkspaceSnapshot(Array.from({ length: 6 }, (_, index) => createSavedSession(index + 1)));

    const controls = resolveTerminalSavedSessionsControlState(snapshot, createOptions({
      visibleSavedSessionCount: 6,
    }));

    expect(controls.visibleCount).toBe(6);
    expect(controls.hiddenCount).toBe(0);
    expect(controls.canShowMore).toBe(false);
    expect(controls.canCollapse).toBe(true);
    expect(controls.canPruneHidden).toBe(false);
  });

  it("blocks restore for incompatible saved sessions with a clear status", () => {
    const snapshot = createWorkspaceSnapshot([
      createSavedSession(1, {
        canRestore: false,
        status: "protocol_minor_ahead",
      }),
    ]);

    const controls = resolveTerminalSavedSessionsControlState(snapshot, createOptions());
    const item = controls.items[0]!;

    expect(item.canRestore).toBe(false);
    expect(item.canDelete).toBe(true);
    expect(item.restoreStatus).toBe("blocked");
    expect(item.restoreTitle).toContain("Cannot restore");
    expect(item.restoreTitle).toContain("newer protocol");
    expect(findRestorableSavedSession(snapshot, createOptions(), "saved-1")).toBeNull();
  });

  it("surfaces restore semantics explicitly for degraded saved layouts", () => {
    const snapshot = createWorkspaceSnapshot([
      createSavedSession(1, {
        restoreSemantics: {
          restores_focus_state: false,
          restores_tab_titles: false,
          uses_saved_launch_spec: false,
          replays_saved_screen_buffers: false,
          preserves_process_state: false,
        },
      }),
    ]);

    const controls = resolveTerminalSavedSessionsControlState(snapshot, createOptions());

    expect(controls.items[0]?.restoreSemanticsNotes.map((note) => note.code)).toEqual([
      "topology_restored",
      "focus_not_restored",
      "tab_titles_not_restored",
      "launch_spec_unavailable",
      "process_state_not_preserved",
      "screen_buffers_not_replayed",
    ]);
    expect(controls.items[0]?.restoreSemanticsNotes.map((note) => note.tone)).toEqual([
      "ok",
      "info",
      "info",
      "warning",
      "warning",
      "info",
    ]);
  });

  it("uses compact ids for untitled saved layout fallbacks", () => {
    const session = createSavedSession(1, {
      sessionId: "d5bcf588-f6ba-46f9-a9b2-d77e6f7258cd-saved-1",
      title: null,
    });
    const snapshot = createWorkspaceSnapshot([session]);

    const controls = resolveTerminalSavedSessionsControlState(snapshot, createOptions());

    expect(controls.items[0]?.title).toBe("d5bcf588...aved-1");
    expect(controls.items[0]?.restoreTitle).toContain("d5bcf588...aved-1");
    expect(controls.items[0]?.restoreTitle).not.toContain(session.session_id);
  });

  it("marks missing topology restore as a warning", () => {
    const snapshot = createWorkspaceSnapshot([
      createSavedSession(1, {
        restoreSemantics: {
          restores_topology: false,
          replays_saved_screen_buffers: true,
          preserves_process_state: true,
        },
      }),
    ]);

    const controls = resolveTerminalSavedSessionsControlState(snapshot, createOptions());

    expect(controls.items[0]?.restoreSemanticsNotes).toEqual([
      {
        code: "topology_not_restored",
        label: "topology unavailable",
        detail: "Pane and tab topology is not restored by this saved layout.",
        tone: "warning",
      },
    ]);
  });

  it("disables item and prune actions while a saved session action is pending", () => {
    const snapshot = createWorkspaceSnapshot(Array.from({ length: 5 }, (_, index) => createSavedSession(index + 1)));

    const controls = resolveTerminalSavedSessionsControlState(snapshot, createOptions({
      pendingSavedSessionId: "saved-1",
      pendingSavedSessionAction: "restore",
    }));

    expect(controls.anyPending).toBe(true);
    expect(controls.canPruneHidden).toBe(false);
    expect(controls.items[0]?.isRestoring).toBe(true);
    expect(controls.items[0]?.canRestore).toBe(false);
    expect(controls.items[0]?.canDelete).toBe(false);
    expect(controls.items[1]?.canRestore).toBe(false);
    expect(controls.items[1]?.canDelete).toBe(false);
    expect(controls.items.every((item) => item.restoreStatus === "pending")).toBe(true);
    expect(findRestorableSavedSession(snapshot, createOptions({
      pendingSavedSessionId: "saved-1",
      pendingSavedSessionAction: "restore",
    }), "saved-2")).toBeNull();
  });

  it("tracks destructive confirmation on the targeted saved session only", () => {
    const snapshot = createWorkspaceSnapshot([
      createSavedSession(1),
      createSavedSession(2),
      createSavedSession(3),
    ]);

    const controls = resolveTerminalSavedSessionsControlState(snapshot, createOptions({
      deleteConfirmationSessionId: "saved-2",
      pruneConfirmationArmed: true,
    }));

    expect(controls.items.map((item) => item.isConfirmingDelete)).toEqual([false, true, false]);
    expect(controls.pruneConfirmationArmed).toBe(true);
    expect(hasSavedSession(snapshot, "saved-2")).toBe(true);
    expect(hasSavedSession(snapshot, "missing")).toBe(false);
  });

  it("finds restorable saved sessions outside the currently rendered page", () => {
    const snapshot = createWorkspaceSnapshot(Array.from({ length: 6 }, (_, index) => createSavedSession(index + 1)));

    const target = findRestorableSavedSession(snapshot, createOptions(), "saved-6");

    expect(target?.session_id).toBe("saved-6");
  });
});

function createOptions(
  overrides: Partial<TerminalSavedSessionsControlOptions> = {},
): TerminalSavedSessionsControlOptions {
  return {
    visibleSavedSessionCount: 4,
    pendingSavedSessionId: null,
    pendingSavedSessionAction: null,
    pendingBulkAction: null,
    deleteConfirmationSessionId: null,
    pruneConfirmationArmed: false,
    ...overrides,
  };
}

function createWorkspaceSnapshot(savedSessions: SavedSessionSummary[]): WorkspaceSnapshot {
  const base = createInitialWorkspaceSnapshot();
  return {
    ...base,
    catalog: {
      ...base.catalog,
      savedSessions,
    },
  };
}

function createSavedSession(
  index: number,
  overrides: {
    canRestore?: boolean;
    status?: SavedSessionCompatibilityStatus;
    restoreSemantics?: Partial<SavedSessionSummary["restore_semantics"]>;
    sessionId?: string;
    title?: string | null;
  } = {},
): SavedSessionSummary {
  return {
    session_id: overrides.sessionId ?? `saved-${index}`,
    route: {
      backend: "native",
      authority: "local_daemon",
      external: null,
    },
    title: overrides.title === undefined ? `Saved ${index}` : overrides.title,
    saved_at_ms: BigInt(1_700_000_000_000 + index),
    manifest: {
      format_version: 1,
      binary_version: "0.1.0-test",
      protocol_major: 0,
      protocol_minor: 2,
    },
    compatibility: {
      can_restore: overrides.canRestore ?? true,
      status: overrides.status ?? "compatible",
    },
    has_launch: true,
    tab_count: 1,
    pane_count: 1,
    restore_semantics: {
      restores_topology: true,
      restores_focus_state: true,
      restores_tab_titles: true,
      uses_saved_launch_spec: true,
      replays_saved_screen_buffers: false,
      preserves_process_state: false,
      ...overrides.restoreSemantics,
    },
  };
}
