import type {
  TerminalBackendKind,
  TerminalCreateNativeSessionInput,
  TerminalMuxCommand,
} from "@features/terminal-workspace-kernel/contracts";
import type { TerminalRuntimeSessionRoute } from "../TerminalRuntimeModels.js";
import type { TerminalWorkspaceControlRuntimePort } from "../ports/TerminalWorkspaceControlRuntimePort.js";

export class TerminalRuntimeControlService {
  readonly #runtime: TerminalWorkspaceControlRuntimePort;

  constructor(runtime: TerminalWorkspaceControlRuntimePort) {
    this.#runtime = runtime;
  }

  handshakeInfo() {
    return this.#runtime.handshakeInfo();
  }

  listSessions() {
    return this.#runtime.listSessions();
  }

  listSavedSessions() {
    return this.#runtime.listSavedSessions();
  }

  discoverSessions(backend: TerminalBackendKind) {
    return this.#runtime.discoverSessions(backend);
  }

  backendCapabilities(backend: TerminalBackendKind) {
    return this.#runtime.backendCapabilities(backend);
  }

  createNativeSession(input: TerminalCreateNativeSessionInput) {
    return this.#runtime.createNativeSession(input);
  }

  importSession(input: {
    route: TerminalRuntimeSessionRoute;
    title?: string;
  }) {
    return this.#runtime.importSession(input);
  }

  restoreSavedSession(sessionId: string) {
    return this.#runtime.restoreSavedSession(sessionId);
  }

  deleteSavedSession(sessionId: string) {
    return this.#runtime.deleteSavedSession(sessionId);
  }

  dispatchMuxCommand(sessionId: string, command: TerminalMuxCommand) {
    return this.#runtime.dispatchMuxCommand(sessionId, command);
  }
}
