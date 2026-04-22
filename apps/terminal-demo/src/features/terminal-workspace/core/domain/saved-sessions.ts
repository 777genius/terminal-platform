import type { TerminalSavedSessionSummary } from "../../contracts/terminal-workspace-contracts.js";

export const SAVED_SESSIONS_COLLAPSED_LIMIT = 10;

export function getVisibleSavedSessions(
  sessions: TerminalSavedSessionSummary[],
  showAll: boolean,
): TerminalSavedSessionSummary[] {
  return showAll ? sessions : sessions.slice(0, SAVED_SESSIONS_COLLAPSED_LIMIT);
}

export function getHiddenSavedSessionsCount(
  allSessions: TerminalSavedSessionSummary[],
  visibleSessions: TerminalSavedSessionSummary[],
): number {
  return Math.max(allSessions.length - visibleSessions.length, 0);
}
