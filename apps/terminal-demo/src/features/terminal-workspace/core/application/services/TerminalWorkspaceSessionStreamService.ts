import type { TerminalSessionState } from "../../../contracts/terminal-workspace-contracts.js";
import type { TerminalWorkspaceSessionStateRuntimePort } from "../ports/TerminalWorkspaceSessionStateRuntimePort.js";

export class TerminalWorkspaceSessionStreamService {
  readonly #runtime: TerminalWorkspaceSessionStateRuntimePort;

  constructor(runtime: TerminalWorkspaceSessionStateRuntimePort) {
    this.#runtime = runtime;
  }

  watchSessionState(
    sessionId: string,
    handlers: {
      onState(state: TerminalSessionState): void;
      onError(error: unknown): void;
      onClosed(): void;
    },
  ) {
    return this.#runtime.watchSessionState(sessionId, handlers);
  }
}
