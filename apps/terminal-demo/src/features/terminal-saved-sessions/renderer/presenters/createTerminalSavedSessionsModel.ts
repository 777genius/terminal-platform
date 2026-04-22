import type {
  TerminalDegradedReason,
  TerminalRuntimeWorkspaceFacade,
  TerminalSavedSessionSummary,
} from "@features/terminal-workspace-kernel/contracts";
import {
  getHiddenSavedSessionsCount,
  getVisibleSavedSessions,
} from "@features/terminal-workspace-kernel/contracts";
import type {
  TerminalSavedSessionItemModel,
  TerminalSavedSessionsDegradedReasonModel,
  TerminalSavedSessionsModel,
} from "../view-models/TerminalSavedSessionsModel.js";

export function createTerminalSavedSessionsModel(input: {
  runtime: TerminalRuntimeWorkspaceFacade;
  showAll: boolean;
}): TerminalSavedSessionsModel {
  const visibleSavedSessions = getVisibleSavedSessions(input.runtime.state.savedSessions, input.showAll);

  return {
    savedSessionItems: visibleSavedSessions.map(toSavedSessionItemModel),
    hiddenSavedSessionsCount: getHiddenSavedSessionsCount(
      input.runtime.state.savedSessions,
      visibleSavedSessions,
    ),
    showAllSavedSessions: input.showAll,
  };
}

function toSavedSessionItemModel(session: TerminalSavedSessionSummary): TerminalSavedSessionItemModel {
  return {
    sessionId: session.session_id,
    title: session.title ?? "Untitled save",
    meta: `${session.origin.backend} - ${new Date(session.saved_at_ms).toLocaleString()}`,
    degradedReasons: toDegradedReasonModels(session.degradedSemantics),
    canRestore: session.compatibility.can_restore,
  };
}

function toDegradedReasonModels(reasons: TerminalDegradedReason[]): TerminalSavedSessionsDegradedReasonModel[] {
  return reasons.map((reason, index) => ({
    id: `${reason.code}-${index}`,
    badge: {
      label: reason.severity,
      tone: reason.severity === "error" ? "danger" : reason.severity === "warning" ? "neutral" : "brand",
    },
    detail: reason.detail,
  }));
}
