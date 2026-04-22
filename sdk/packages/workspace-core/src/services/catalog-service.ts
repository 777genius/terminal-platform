import { toWorkspaceError } from "@terminal-platform/workspace-contracts";

import type { BackendKind } from "@terminal-platform/runtime-types";

import type { ServiceContext } from "./service-context.js";

export class CatalogService {
  readonly #context: ServiceContext;

  constructor(context: ServiceContext) {
    this.#context = context;
  }

  async refreshSessions(): Promise<void> {
    try {
      const transport = await this.#context.ensureTransport();
      const sessions = await transport.listSessions();

      this.#context.updateSnapshot((snapshot) => ({
        ...snapshot,
        catalog: {
          ...snapshot.catalog,
          sessions,
        },
      }));
    } catch (error) {
      throw this.#handleTransportError(error, "failed to refresh sessions");
    }
  }

  async refreshSavedSessions(): Promise<void> {
    try {
      const transport = await this.#context.ensureTransport();
      const savedSessions = await transport.listSavedSessions();

      this.#context.updateSnapshot((snapshot) => ({
        ...snapshot,
        catalog: {
          ...snapshot.catalog,
          savedSessions,
        },
      }));
    } catch (error) {
      throw this.#handleTransportError(error, "failed to refresh saved sessions");
    }
  }

  async discoverSessions(backend: BackendKind): Promise<void> {
    try {
      const transport = await this.#context.ensureTransport();
      const sessions = await transport.discoverSessions(backend);

      this.#context.updateSnapshot((snapshot) => ({
        ...snapshot,
        catalog: {
          ...snapshot.catalog,
          discoveredSessions: {
            ...snapshot.catalog.discoveredSessions,
            [backend]: sessions,
          },
        },
      }));
    } catch (error) {
      throw this.#handleTransportError(error, "failed to discover sessions");
    }
  }

  async getBackendCapabilities(backend: BackendKind) {
    try {
      const transport = await this.#context.ensureTransport();
      const info = await transport.getBackendCapabilities(backend);

      this.#context.updateSnapshot((snapshot) => ({
        ...snapshot,
        catalog: {
          ...snapshot.catalog,
          backendCapabilities: {
            ...snapshot.catalog.backendCapabilities,
            [backend]: info,
          },
        },
      }));

      return info;
    } catch (error) {
      throw this.#handleTransportError(error, "failed to load backend capabilities");
    }
  }

  #handleTransportError(error: unknown, message: string) {
    const workspaceError = toWorkspaceError(error, {
      code: "transport_failed",
      message,
      recoverable: true,
    });

    this.#context.recordDiagnostic({
      code: workspaceError.code,
      message: workspaceError.message,
      severity: "error",
      recoverable: workspaceError.recoverable,
      cause: workspaceError.cause,
    });

    return workspaceError;
  }
}
