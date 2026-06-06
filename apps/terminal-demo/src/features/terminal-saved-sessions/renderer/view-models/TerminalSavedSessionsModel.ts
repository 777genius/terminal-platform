export type TerminalSavedSessionsTone = "brand" | "neutral" | "danger";

export interface TerminalSavedSessionsBadgeModel {
  label: string;
  tone: TerminalSavedSessionsTone;
}

export interface TerminalSavedSessionsDegradedReasonModel {
  id: string;
  badge: TerminalSavedSessionsBadgeModel;
  detail: string;
}

export interface TerminalSavedSessionItemModel {
  sessionId: string;
  title: string;
  meta: string;
  restoreGuaranteeBadge: TerminalSavedSessionsBadgeModel | null;
  degradedReasons: TerminalSavedSessionsDegradedReasonModel[];
  canRestore: boolean;
}

export interface TerminalSavedSessionsModel {
  savedSessionItems: TerminalSavedSessionItemModel[];
  hiddenSavedSessionsCount: number;
  showAllSavedSessions: boolean;
}
