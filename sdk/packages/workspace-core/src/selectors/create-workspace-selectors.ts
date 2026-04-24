import type { WorkspaceSelectors } from "../kernel/types.js";
import type { WorkspaceSnapshot } from "../read-models/workspace-snapshot.js";

export function createWorkspaceSelectors(
  getSnapshot: () => WorkspaceSnapshot,
): WorkspaceSelectors {
  return {
    connection() {
      return getSnapshot().connection;
    },
    sessions() {
      return getSnapshot().catalog.sessions;
    },
    savedSessions() {
      return getSnapshot().catalog.savedSessions;
    },
    activeSession() {
      const snapshot = getSnapshot();
      return (
        snapshot.catalog.sessions.find(
          (session) => session.session_id === snapshot.selection.activeSessionId,
        ) ?? null
      );
    },
    activePaneId() {
      return getSnapshot().selection.activePaneId;
    },
    attachedSession() {
      return getSnapshot().attachedSession;
    },
    diagnostics() {
      return getSnapshot().diagnostics;
    },
    themeId() {
      return getSnapshot().theme.themeId;
    },
    terminalDisplay() {
      return getSnapshot().terminalDisplay;
    },
  };
}
