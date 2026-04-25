import { AsyncLane } from "@terminal-platform/foundation";
import { toWorkspaceError, WorkspaceError } from "@terminal-platform/workspace-contracts";

import type {
  BackendKind,
  CreateSessionRequest,
  MuxCommand,
  MuxCommandResult,
  PaneId,
  PruneSavedSessionsResult,
  SessionId,
  SessionRoute,
  SubscriptionSpec,
} from "@terminal-platform/runtime-types";
import type { WorkspaceSubscription } from "@terminal-platform/workspace-contracts";

import type { WorkspaceSnapshot } from "../read-models/workspace-snapshot.js";
import type { CatalogService } from "./catalog-service.js";
import type { ServiceContext } from "./service-context.js";

export class SessionCommandService {
  readonly #context: ServiceContext;
  readonly #catalogService: CatalogService;
  readonly #lane = new AsyncLane();

  constructor(context: ServiceContext, catalogService: CatalogService) {
    this.#context = context;
    this.#catalogService = catalogService;
  }

  createSession(backend: BackendKind, request: CreateSessionRequest): Promise<void> {
    return this.#lane.enqueue(async () => {
      try {
        const transport = await this.#context.ensureTransport();
        const session = await transport.createSession(backend, request);

        this.#context.updateSnapshot((snapshot) => ({
          ...snapshot,
          catalog: {
            ...snapshot.catalog,
            sessions: mergeSession(snapshot.catalog.sessions, session),
          },
          selection: {
            ...snapshot.selection,
            activeSessionId: session.session_id,
          },
        }));
      } catch (error) {
        throw this.#handleTransportError(error, "failed to create session");
      }
    });
  }

  importSession(route: SessionRoute, title?: string | null): Promise<void> {
    return this.#lane.enqueue(async () => {
      try {
        const transport = await this.#context.ensureTransport();
        const session = await transport.importSession(route, title);

        this.#context.updateSnapshot((snapshot) => ({
          ...snapshot,
          catalog: {
            ...snapshot.catalog,
            sessions: mergeSession(snapshot.catalog.sessions, session),
          },
          selection: {
            ...snapshot.selection,
            activeSessionId: session.session_id,
          },
        }));
      } catch (error) {
        throw this.#handleTransportError(error, "failed to import session");
      }
    });
  }

  attachSession(sessionId: SessionId): Promise<void> {
    return this.#lane.enqueue(async () => {
      try {
        const transport = await this.#context.ensureTransport();
        const attachedSession = await transport.attachSession(sessionId);

        this.#context.updateSnapshot((snapshot) => ({
          ...snapshot,
          attachedSession,
          selection: {
            activeSessionId: attachedSession.session.session_id,
            activePaneId: attachedSession.focused_screen?.pane_id ?? null,
          },
        }));
      } catch (error) {
        throw this.#handleTransportError(error, "failed to attach session");
      }
    });
  }

  restoreSavedSession(sessionId: SessionId): Promise<void> {
    return this.#lane.enqueue(async () => {
      try {
        const restoreBlocker = findSavedSessionRestoreBlocker(this.#context.getSnapshot(), sessionId);
        if (restoreBlocker) {
          throw restoreBlocker;
        }

        const transport = await this.#context.ensureTransport();
        const restored = await transport.restoreSavedSession(sessionId);

        this.#context.updateSnapshot((snapshot) => ({
          ...snapshot,
          catalog: {
            ...snapshot.catalog,
            sessions: mergeSession(snapshot.catalog.sessions, restored.session),
          },
          selection: {
            ...snapshot.selection,
            activeSessionId: restored.session.session_id,
          },
        }));

        await this.#catalogService.refreshSavedSessions();
      } catch (error) {
        throw this.#handleTransportError(error, "failed to restore saved session");
      }
    });
  }

  deleteSavedSession(sessionId: SessionId): Promise<void> {
    return this.#lane.enqueue(async () => {
      try {
        const transport = await this.#context.ensureTransport();
        await transport.deleteSavedSession(sessionId);
        await this.#catalogService.refreshSavedSessions();
      } catch (error) {
        throw this.#handleTransportError(error, "failed to delete saved session");
      }
    });
  }

  pruneSavedSessions(keepLatest: number): Promise<PruneSavedSessionsResult> {
    return this.#lane.enqueue(async () => {
      try {
        const transport = await this.#context.ensureTransport();
        const result = await transport.pruneSavedSessions(keepLatest);
        await this.#catalogService.refreshSavedSessions();
        return result;
      } catch (error) {
        throw this.#handleTransportError(error, "failed to prune saved sessions");
      }
    });
  }

  dispatchMuxCommand(sessionId: SessionId, command: MuxCommand): Promise<MuxCommandResult> {
    return this.#lane.enqueue(async () => {
      try {
        const transport = await this.#context.ensureTransport();
        return await transport.dispatchMuxCommand(sessionId, command);
      } catch (error) {
        throw this.#handleTransportError(error, "failed to dispatch mux command");
      }
    });
  }

  async openSubscription(
    sessionId: SessionId,
    spec: SubscriptionSpec,
  ): Promise<WorkspaceSubscription> {
    try {
      const transport = await this.#context.ensureTransport();
      return await transport.openSubscription(sessionId, spec);
    } catch (error) {
      throw this.#handleTransportError(error, "failed to open subscription");
    }
  }

  setActiveSession(sessionId: SessionId | null): void {
    this.#context.updateSnapshot((snapshot) => ({
      ...snapshot,
      selection: {
        ...snapshot.selection,
        activeSessionId: sessionId,
      },
    }));
  }

  setActivePane(paneId: PaneId | null): void {
    this.#context.updateSnapshot((snapshot) => ({
      ...snapshot,
      selection: {
        ...snapshot.selection,
        activePaneId: paneId,
      },
    }));
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

function findSavedSessionRestoreBlocker(
  snapshot: WorkspaceSnapshot,
  sessionId: SessionId,
): WorkspaceError | null {
  const savedSession = snapshot.catalog.savedSessions.find((candidate) => candidate.session_id === sessionId);
  if (!savedSession || savedSession.compatibility.can_restore) {
    return null;
  }

  return new WorkspaceError({
    code: "unsupported_capability",
    message: `saved session ${sessionId} is not restore-compatible: ${savedSession.compatibility.status}`,
    recoverable: false,
  });
}

function mergeSession<TSession extends { session_id: string }>(
  sessions: readonly TSession[],
  nextSession: TSession,
): TSession[] {
  const remaining = sessions.filter((session) => session.session_id !== nextSession.session_id);
  return [...remaining, nextSession];
}
