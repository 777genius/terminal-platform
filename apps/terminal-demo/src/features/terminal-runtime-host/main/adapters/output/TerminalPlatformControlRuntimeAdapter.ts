import type {
  TerminalBackendKind,
  TerminalCreateNativeSessionInput,
  TerminalMuxCommand,
} from "@features/terminal-workspace-kernel/contracts";
import type {
  TerminalWorkspaceControlRuntimePort,
  TerminalRuntimeSessionRoute,
} from "../../../core/application/index.js";
import type { TerminalPlatformClientProvider } from "../../infrastructure/TerminalPlatformClientProvider.js";
import {
  toSdkCreateNativeSessionRequest,
  toSdkImportSessionInput,
  toSdkMuxCommand,
  toTerminalBackendCapabilitiesInfo,
  toTerminalDeleteSavedSessionResponse,
  toTerminalHandshakeInfo,
  toTerminalMuxCommandResult,
  toTerminalRuntimeDiscoveredSession,
  toTerminalSavedSessionSummary,
  toTerminalSessionSummary,
} from "./terminal-platform-contract-mappers.js";

export class TerminalPlatformControlRuntimeAdapter implements TerminalWorkspaceControlRuntimePort {
  readonly #clientProvider: TerminalPlatformClientProvider;

  constructor(clientProvider: TerminalPlatformClientProvider) {
    this.#clientProvider = clientProvider;
  }

  async handshakeInfo() {
    const client = await this.#clientProvider.getClient();
    return toTerminalHandshakeInfo(await client.handshakeInfo());
  }

  async listSessions() {
    const client = await this.#clientProvider.getClient();
    return (await client.listSessions()).map(toTerminalSessionSummary);
  }

  async listSavedSessions() {
    const client = await this.#clientProvider.getClient();
    return (await client.listSavedSessions()).map(toTerminalSavedSessionSummary);
  }

  async discoverSessions(backend: TerminalBackendKind) {
    const client = await this.#clientProvider.getClient();
    return (await client.discoverSessions(backend)).map(toTerminalRuntimeDiscoveredSession);
  }

  async backendCapabilities(backend: TerminalBackendKind) {
    const client = await this.#clientProvider.getClient();
    return toTerminalBackendCapabilitiesInfo(await client.backendCapabilities(backend));
  }

  async createNativeSession(input: TerminalCreateNativeSessionInput) {
    const client = await this.#clientProvider.getClient();
    return toTerminalSessionSummary(
      await client.createNativeSession(toSdkCreateNativeSessionRequest(input)),
    );
  }

  async importSession(input: {
    route: TerminalRuntimeSessionRoute;
    title?: string;
  }) {
    const client = await this.#clientProvider.getClient();
    const request = toSdkImportSessionInput(input);
    return toTerminalSessionSummary(
      await client.importSession(request.route, request.title ?? null),
    );
  }

  async restoreSavedSession(sessionId: string) {
    const client = await this.#clientProvider.getClient();
    const restored = await client.restoreSavedSession(sessionId);
    return toTerminalSessionSummary(restored.session);
  }

  async deleteSavedSession(sessionId: string) {
    const client = await this.#clientProvider.getClient();
    return toTerminalDeleteSavedSessionResponse(await client.deleteSavedSession(sessionId));
  }

  async dispatchMuxCommand(sessionId: string, command: TerminalMuxCommand) {
    const client = await this.#clientProvider.getClient();
    return toTerminalMuxCommandResult(
      await client.dispatchMuxCommand(sessionId, toSdkMuxCommand(command)),
    );
  }
}
