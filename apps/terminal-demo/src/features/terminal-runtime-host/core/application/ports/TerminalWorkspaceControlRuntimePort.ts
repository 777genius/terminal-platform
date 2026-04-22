import type {
  TerminalBackendCapabilitiesInfo,
  TerminalBackendKind,
  TerminalCreateNativeSessionInput,
  TerminalDeleteSavedSessionResponse,
  TerminalHandshakeInfo,
  TerminalMuxCommand,
  TerminalMuxCommandResult,
  TerminalSavedSessionSummary,
  TerminalSessionSummary,
} from "@features/terminal-workspace-kernel/contracts";
import type {
  TerminalRuntimeDiscoveredSession,
  TerminalRuntimeSessionRoute,
} from "../TerminalRuntimeModels.js";

export interface TerminalWorkspaceControlRuntimePort {
  handshakeInfo(): Promise<TerminalHandshakeInfo>;
  listSessions(): Promise<TerminalSessionSummary[]>;
  listSavedSessions(): Promise<TerminalSavedSessionSummary[]>;
  discoverSessions(backend: TerminalBackendKind): Promise<TerminalRuntimeDiscoveredSession[]>;
  backendCapabilities(backend: TerminalBackendKind): Promise<TerminalBackendCapabilitiesInfo>;
  createNativeSession(input: TerminalCreateNativeSessionInput): Promise<TerminalSessionSummary>;
  importSession(input: {
    route: TerminalRuntimeSessionRoute;
    title?: string;
  }): Promise<TerminalSessionSummary>;
  restoreSavedSession(sessionId: string): Promise<TerminalSessionSummary>;
  deleteSavedSession(sessionId: string): Promise<TerminalDeleteSavedSessionResponse>;
  dispatchMuxCommand(sessionId: string, command: TerminalMuxCommand): Promise<TerminalMuxCommandResult>;
}
