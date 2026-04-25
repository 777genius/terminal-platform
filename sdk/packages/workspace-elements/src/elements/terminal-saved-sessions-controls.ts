import type { SavedSessionSummary } from "@terminal-platform/runtime-types";
import type { WorkspaceSnapshot } from "@terminal-platform/workspace-core";

export type TerminalSavedSessionPendingAction = "restore" | "delete";
export type TerminalSavedSessionsBulkAction = "prune";
export type TerminalSavedSessionRestoreStatus = "available" | "blocked" | "pending";

export interface TerminalSavedSessionsControlOptions {
  visibleSavedSessionCount: number;
  pendingSavedSessionId: string | null;
  pendingSavedSessionAction: TerminalSavedSessionPendingAction | null;
  pendingBulkAction: TerminalSavedSessionsBulkAction | null;
  deleteConfirmationSessionId: string | null;
  pruneConfirmationArmed: boolean;
}

export interface TerminalSavedSessionItemControlState {
  session: SavedSessionSummary;
  title: string;
  compatibilityLabel: string;
  isPending: boolean;
  isRestoring: boolean;
  isDeleting: boolean;
  isConfirmingDelete: boolean;
  canRestore: boolean;
  canDelete: boolean;
  restoreStatus: TerminalSavedSessionRestoreStatus;
  restoreTitle: string;
}

export interface TerminalSavedSessionsControlState {
  savedSessionCount: number;
  visibleCount: number;
  hiddenCount: number;
  anyPending: boolean;
  isPruning: boolean;
  canShowMore: boolean;
  canCollapse: boolean;
  canPruneHidden: boolean;
  pruneKeepLatest: number;
  pruneConfirmationArmed: boolean;
  items: TerminalSavedSessionItemControlState[];
}

export const TERMINAL_SAVED_SESSIONS_DEFAULT_VISIBLE_COUNT = 4;

export function resolveTerminalSavedSessionsControlState(
  snapshot: WorkspaceSnapshot,
  options: TerminalSavedSessionsControlOptions,
): TerminalSavedSessionsControlState {
  const savedSessions = snapshot.catalog.savedSessions;
  const visibleCount = clampVisibleSavedSessionCount(options.visibleSavedSessionCount, savedSessions.length);
  const hiddenCount = savedSessions.length - visibleCount;
  const anyPending = Boolean(options.pendingSavedSessionId || options.pendingBulkAction);
  const isPruning = options.pendingBulkAction === "prune";
  const pruneKeepLatest = visibleCount;

  return {
    savedSessionCount: savedSessions.length,
    visibleCount,
    hiddenCount,
    anyPending,
    isPruning,
    canShowMore: hiddenCount > 0,
    canCollapse: visibleCount > TERMINAL_SAVED_SESSIONS_DEFAULT_VISIBLE_COUNT,
    canPruneHidden: hiddenCount > 0 && !anyPending,
    pruneKeepLatest,
    pruneConfirmationArmed: options.pruneConfirmationArmed,
    items: savedSessions.slice(0, visibleCount).map((session) => toSavedSessionItemControlState(session, options, anyPending)),
  };
}

export function findRestorableSavedSession(
  snapshot: WorkspaceSnapshot,
  options: TerminalSavedSessionsControlOptions,
  sessionId: string,
): SavedSessionSummary | null {
  const controls = resolveTerminalSavedSessionsControlState(snapshot, {
    ...options,
    visibleSavedSessionCount: Math.max(options.visibleSavedSessionCount, snapshot.catalog.savedSessions.length),
  });
  return controls.items.find((item) => item.session.session_id === sessionId && item.canRestore)?.session ?? null;
}

export function hasSavedSession(
  snapshot: WorkspaceSnapshot,
  sessionId: string,
): boolean {
  return snapshot.catalog.savedSessions.some((session) => session.session_id === sessionId);
}

function toSavedSessionItemControlState(
  session: SavedSessionSummary,
  options: TerminalSavedSessionsControlOptions,
  anyPending: boolean,
): TerminalSavedSessionItemControlState {
  const title = session.title ?? session.session_id;
  const isPending = options.pendingSavedSessionId === session.session_id;
  const isRestoring = isPending && options.pendingSavedSessionAction === "restore";
  const isDeleting = isPending && options.pendingSavedSessionAction === "delete";
  const canRestore = Boolean(!anyPending && session.compatibility.can_restore);
  const canDelete = !anyPending;

  return {
    session,
    title,
    compatibilityLabel: savedSessionCompatibilityLabel(session),
    isPending,
    isRestoring,
    isDeleting,
    isConfirmingDelete: options.deleteConfirmationSessionId === session.session_id,
    canRestore,
    canDelete,
    restoreStatus: resolveRestoreStatus(session, anyPending),
    restoreTitle: restoreButtonTitle(session, title, anyPending),
  };
}

function clampVisibleSavedSessionCount(visibleCount: number, savedSessionCount: number): number {
  if (!Number.isFinite(visibleCount)) {
    return Math.min(TERMINAL_SAVED_SESSIONS_DEFAULT_VISIBLE_COUNT, savedSessionCount);
  }

  return Math.min(savedSessionCount, Math.max(0, Math.trunc(visibleCount)));
}

function resolveRestoreStatus(
  session: SavedSessionSummary,
  anyPending: boolean,
): TerminalSavedSessionRestoreStatus {
  if (anyPending) {
    return "pending";
  }

  return session.compatibility.can_restore ? "available" : "blocked";
}

function restoreButtonTitle(session: SavedSessionSummary, title: string, anyPending: boolean): string {
  if (anyPending) {
    return "Wait for the current saved layout action to finish.";
  }

  const compatibilityLabel = savedSessionCompatibilityLabel(session);
  if (!session.compatibility.can_restore) {
    return `Cannot restore "${title}". ${compatibilityLabel}.`;
  }

  return `Restore saved layout "${title}". ${compatibilityLabel}.`;
}

function savedSessionCompatibilityLabel(session: SavedSessionSummary): string {
  switch (session.compatibility.status) {
    case "compatible":
      return "Compatible with this runtime";
    case "binary_skew":
      return "Compatible with a different binary version";
    case "format_version_unsupported":
      return "Saved layout format is unsupported";
    case "protocol_major_unsupported":
      return "Saved layout protocol major version is unsupported";
    case "protocol_minor_ahead":
      return "Saved layout was created by a newer protocol";
    default:
      return "Saved layout compatibility is unknown";
  }
}
