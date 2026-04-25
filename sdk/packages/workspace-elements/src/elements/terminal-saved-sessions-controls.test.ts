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
  } = {},
): SavedSessionSummary {
  return {
    session_id: `saved-${index}`,
    route: {
      backend: "native",
      authority: "local_daemon",
      external: null,
    },
    title: `Saved ${index}`,
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
    },
  };
}
