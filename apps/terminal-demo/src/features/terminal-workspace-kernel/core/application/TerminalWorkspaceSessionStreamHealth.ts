export type TerminalWorkspaceSessionStreamPhase =
  | "idle"
  | "connecting"
  | "ready"
  | "reconnecting"
  | "error";

export interface TerminalWorkspaceSessionStreamHealth {
  phase: TerminalWorkspaceSessionStreamPhase;
  reconnectAttempts: number;
  lastError: string | null;
}

export const initialTerminalWorkspaceSessionStreamHealth: TerminalWorkspaceSessionStreamHealth = {
  phase: "idle",
  reconnectAttempts: 0,
  lastError: null,
};
