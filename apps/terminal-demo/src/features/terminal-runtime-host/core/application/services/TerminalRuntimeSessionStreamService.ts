import type { TerminalSessionState } from "@features/terminal-workspace-kernel/contracts";
import type { TerminalWorkspaceSessionStateRuntimePort } from "../ports/TerminalWorkspaceSessionStateRuntimePort.js";

export class TerminalRuntimeSessionStreamService {
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
