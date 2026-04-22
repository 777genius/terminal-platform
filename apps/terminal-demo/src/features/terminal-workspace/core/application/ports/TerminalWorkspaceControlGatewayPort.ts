import type {
  TerminalBackendCapabilitiesInfo,
  TerminalBackendKind,
  TerminalCreateNativeSessionInput,
  TerminalDeleteSavedSessionResponse,
  TerminalDiscoveredSession,
  TerminalHandshakeInfo,
  TerminalImportSessionInput,
  TerminalMuxCommand,
  TerminalMuxCommandResult,
  TerminalSavedSessionSummary,
  TerminalSessionSummary,
} from "../../../contracts/terminal-workspace-contracts.js";

export interface TerminalWorkspaceControlGatewayPort {
  handshakeInfo(): Promise<TerminalHandshakeInfo>;
  listSessions(): Promise<TerminalSessionSummary[]>;
  listSavedSessions(): Promise<TerminalSavedSessionSummary[]>;
  discoverSessions(backend: TerminalBackendKind): Promise<TerminalDiscoveredSession[]>;
  backendCapabilities(backend: TerminalBackendKind): Promise<TerminalBackendCapabilitiesInfo>;
  createNativeSession(input?: TerminalCreateNativeSessionInput): Promise<TerminalSessionSummary>;
  importSession(input: TerminalImportSessionInput): Promise<TerminalSessionSummary>;
  restoreSavedSession(sessionId: string): Promise<TerminalSessionSummary>;
  deleteSavedSession(sessionId: string): Promise<TerminalDeleteSavedSessionResponse>;
  dispatchMuxCommand(sessionId: string, command: TerminalMuxCommand): Promise<TerminalMuxCommandResult>;
  dispose(): void;
}
