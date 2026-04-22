import type {
  TerminalBackendCapabilitiesInfo,
  TerminalDiscoveredSession,
  TerminalDegradedReason,
  TerminalHandshakeInfo,
  TerminalSavedSessionSummary,
  TerminalSessionSummary,
  TerminalSessionState,
  TerminalBackendKind,
} from "../../contracts/terminal-workspace-contracts.js";
import {
  initialTerminalWorkspaceSessionStreamHealth,
  type TerminalWorkspaceSessionStreamHealth,
} from "./TerminalWorkspaceSessionStreamHealth.js";

export interface TerminalWorkspaceViewState {
  status: "idle" | "loading" | "ready" | "error";
  sessionStatus: "idle" | "connecting" | "ready" | "error";
  error: string | null;
  actionError: string | null;
  actionDegradedReason: TerminalDegradedReason | null;
  sessionStreamHealth: TerminalWorkspaceSessionStreamHealth;
  handshake: TerminalHandshakeInfo | null;
  sessions: TerminalSessionSummary[];
  savedSessions: TerminalSavedSessionSummary[];
  discoveredSessions: Partial<Record<TerminalBackendKind, TerminalDiscoveredSession[]>>;
  capabilities: Partial<Record<TerminalBackendKind, TerminalBackendCapabilitiesInfo>>;
  activeSessionId: string | null;
  activeSessionState: TerminalSessionState | null;
  createTitleDraft: string;
  createProgramDraft: string;
  createArgsDraft: string;
  createCwdDraft: string;
  inputDraft: string;
}

export const initialTerminalWorkspaceViewState: TerminalWorkspaceViewState = {
  status: "idle",
  sessionStatus: "idle",
  error: null,
  actionError: null,
  actionDegradedReason: null,
  sessionStreamHealth: initialTerminalWorkspaceSessionStreamHealth,
  handshake: null,
  sessions: [],
  savedSessions: [],
  discoveredSessions: {},
  capabilities: {},
  activeSessionId: null,
  activeSessionState: null,
  createTitleDraft: "Workspace",
  createProgramDraft: "",
  createArgsDraft: "",
  createCwdDraft: "",
  inputDraft: "",
};
