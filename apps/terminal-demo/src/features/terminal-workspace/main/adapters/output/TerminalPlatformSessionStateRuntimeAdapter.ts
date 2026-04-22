import type * as TerminalPlatformSdk from "../../../../../../.generated/terminal-platform-node/index.mjs";
import type { TerminalSessionState } from "../../../contracts/terminal-workspace-contracts.js";
import type { TerminalWorkspaceSessionStateRuntimePort } from "../../../core/application/index.js";
import type { TerminalPlatformClientProvider } from "../../infrastructure/TerminalPlatformClientProvider.js";
import { toTerminalSessionState } from "./terminal-platform-contract-mappers.js";

export class TerminalPlatformSessionStateRuntimeAdapter implements TerminalWorkspaceSessionStateRuntimePort {
  readonly #clientProvider: TerminalPlatformClientProvider;

  constructor(clientProvider: TerminalPlatformClientProvider) {
    this.#clientProvider = clientProvider;
  }

  async watchSessionState(
    sessionId: string,
    handlers: {
      onState(state: TerminalSessionState): void;
      onError(error: unknown): void;
      onClosed(): void;
    },
  ) {
    const client = await this.#clientProvider.getClient();
    const abortController = new AbortController();

    const watchPromise = client
      .watchSessionState(sessionId, {
        signal: abortController.signal,
        onState: async (state: TerminalPlatformSdk.TerminalNodeSessionState) => {
          handlers.onState(toTerminalSessionState(state));
        },
      })
      .catch((error: unknown) => {
        if (!abortController.signal.aborted) {
          handlers.onError(error);
        }
      })
      .finally(() => {
        handlers.onClosed();
      });

    return {
      sessionId,
      dispose: async () => {
        abortController.abort();
        await watchPromise;
      },
    };
  }
}
