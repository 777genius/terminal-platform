import { GenerationToken } from "@terminal-platform/foundation";
import { toWorkspaceError } from "@terminal-platform/workspace-contracts";

import type { ServiceContext } from "./service-context.js";

export class ConnectionService {
  readonly #context: ServiceContext;
  readonly #generation = new GenerationToken();

  constructor(context: ServiceContext) {
    this.#context = context;
  }

  async bootstrap(): Promise<void> {
    const generation = this.#generation.next();

    this.#context.updateSnapshot((snapshot) => ({
      ...snapshot,
      connection: {
        ...snapshot.connection,
        state: "bootstrapping",
        lastError: null,
      },
    }));

    try {
      const transport = await this.#context.ensureTransport();
      const handshake = await transport.handshake();

      if (!this.#generation.isCurrent(generation)) {
        return;
      }

      this.#context.updateSnapshot((snapshot) => ({
        ...snapshot,
        connection: {
          state: "ready",
          handshake,
          lastError: null,
        },
      }));

      this.#context.telemetry.emit({
        name: "workspace.bootstrap.ready",
        attributes: {
          daemonPhase: handshake.daemon_phase,
        },
      });
    } catch (error) {
      const workspaceError = toWorkspaceError(error, {
        code: "bootstrap_failed",
        message: "failed to bootstrap workspace transport",
        recoverable: true,
      });

      if (this.#generation.isCurrent(generation)) {
        this.#context.updateSnapshot((snapshot) => ({
          ...snapshot,
          connection: {
            ...snapshot.connection,
            state: "error",
            lastError: workspaceError,
          },
        }));
      }

      this.#context.recordDiagnostic({
        code: workspaceError.code,
        message: workspaceError.message,
        severity: "error",
        recoverable: workspaceError.recoverable,
        cause: workspaceError.cause,
      });

      throw workspaceError;
    }
  }

  markDisposed(): void {
    this.#generation.next();
    this.#context.updateSnapshot((snapshot) => ({
      ...snapshot,
      connection: {
        ...snapshot.connection,
        state: "disposed",
      },
    }));
  }
}
