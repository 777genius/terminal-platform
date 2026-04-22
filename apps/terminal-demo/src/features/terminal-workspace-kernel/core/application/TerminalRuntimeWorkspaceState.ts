import type {
  TerminalBackendCapabilitiesInfo,
  TerminalBackendKind,
  TerminalDegradedReason,
  TerminalDiscoveredSession,
  TerminalHandshakeInfo,
  TerminalImportSessionInput,
  TerminalSavedSessionSummary,
  TerminalSessionState,
  TerminalSplitDirection,
  TerminalSessionSummary,
} from "../../contracts/terminal-workspace-contracts.js";
import {
  initialTerminalWorkspaceSessionStreamHealth,
  type TerminalWorkspaceSessionStreamHealth,
} from "./TerminalWorkspaceSessionStreamHealth.js";

export interface TerminalRuntimeCreateSessionDraft {
  title: string;
  program: string;
  args: string;
  cwd: string;
}

export interface TerminalRuntimeWorkspaceTransport {
  controlPlaneUrl: string;
  sessionStreamUrl: string;
  runtimeSlug: string;
}

export interface TerminalRuntimeWorkspaceState {
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
}

export const initialTerminalRuntimeWorkspaceState: TerminalRuntimeWorkspaceState = {
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
};

export interface TerminalRuntimeWorkspaceCommands {
  refreshCatalog(): Promise<void>;
  selectSession(sessionId: string): Promise<void>;
  createNativeSession(input: TerminalRuntimeCreateSessionDraft): Promise<void>;
  importSession(input: TerminalImportSessionInput): Promise<void>;
  restoreSavedSession(sessionId: string): Promise<void>;
  deleteSavedSession(sessionId: string): Promise<void>;
  focusPane(paneId: string): Promise<void>;
  focusTab(tabId: string): Promise<void>;
  splitFocusedPane(direction: TerminalSplitDirection): Promise<void>;
  newTab(): Promise<void>;
  saveSession(): Promise<void>;
  submitInput(input: string): Promise<boolean>;
  sendShortcut(data: string): Promise<void>;
}

export interface TerminalRuntimeWorkspaceFacade {
  transport: TerminalRuntimeWorkspaceTransport;
  state: TerminalRuntimeWorkspaceState;
  commands: TerminalRuntimeWorkspaceCommands;
}
