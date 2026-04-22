import { ResourceScope, createExternalStore, noopTelemetrySink } from "@terminal-platform/foundation";

import { CatalogService } from "../services/catalog-service.js";
import { ConnectionService } from "../services/connection-service.js";
import { DiagnosticsService } from "../services/diagnostics-service.js";
import { DraftInputService } from "../services/draft-input-service.js";
import { SessionCommandService } from "../services/session-command-service.js";
import { ThemeResolutionService } from "../services/theme-resolution-service.js";
import { createWorkspaceSelectors } from "../selectors/create-workspace-selectors.js";
import { createInitialWorkspaceSnapshot } from "../read-models/workspace-snapshot.js";

import type { WorkspaceTransportClient, WorkspaceTransportFactory } from "@terminal-platform/workspace-contracts";

import type { CreateWorkspaceKernelOptions, WorkspaceDiagnostics, WorkspaceKernel } from "./types.js";
import type { ServiceContext } from "../services/service-context.js";

export function createWorkspaceKernel(options: CreateWorkspaceKernelOptions): WorkspaceKernel {
  const store = createExternalStore(createInitialWorkspaceSnapshot());
  const scope = new ResourceScope();
  const telemetry = options.telemetry ?? noopTelemetrySink;
  const now = options.now ?? (() => Date.now());

  let disposed = false;
  let transportPromise: Promise<WorkspaceTransportClient> | null = null;

  const clearDiagnostics = () => {
    store.update((snapshot) => ({
      ...snapshot,
      diagnostics: [],
    }));
  };

  const context: ServiceContext = {
    async ensureTransport() {
      assertNotDisposed();

      if (!transportPromise) {
        transportPromise = Promise.resolve(resolveTransport(options.transport)).then((transport) => {
          scope.add(async () => {
            await transport.close();
          });
          return transport;
        });
      }

      return transportPromise;
    },
    getSnapshot: store.getSnapshot,
    updateSnapshot: store.update,
    recordDiagnostic(input) {
      const record = {
        ...input,
        timestampMs: now(),
      };

      store.update((snapshot) => ({
        ...snapshot,
        diagnostics: [...snapshot.diagnostics, record],
      }));

      telemetry.emit({
        name: "workspace.diagnostic.recorded",
        attributes: {
          code: record.code,
          severity: record.severity,
          recoverable: record.recoverable,
        },
      });

      return record;
    },
    clearDiagnostics,
    telemetry,
    now,
  };

  const connectionService = new ConnectionService(context);
  const catalogService = new CatalogService(context);
  const sessionCommandService = new SessionCommandService(context, catalogService);
  const draftInputService = new DraftInputService(context);
  const themeResolutionService = new ThemeResolutionService(context);
  const selectors = createWorkspaceSelectors(store.getSnapshot);
  const diagnostics: WorkspaceDiagnostics = new DiagnosticsService({
    clearDiagnostics,
    getSnapshot: store.getSnapshot,
    now,
    telemetry,
    updateSnapshot: store.update,
  });

  async function bootstrap(): Promise<void> {
    assertNotDisposed();
    await connectionService.bootstrap();
    await catalogService.refreshSessions();
    await catalogService.refreshSavedSessions();
  }

  async function dispose(): Promise<void> {
    if (disposed) {
      return;
    }

    disposed = true;
    connectionService.markDisposed();
    await scope.dispose();
  }

  return {
    getSnapshot: store.getSnapshot,
    subscribe: store.subscribe,
    bootstrap,
    dispose,
    commands: {
      bootstrap,
      refreshSessions: () => catalogService.refreshSessions(),
      refreshSavedSessions: () => catalogService.refreshSavedSessions(),
      discoverSessions: (backend) => catalogService.discoverSessions(backend),
      getBackendCapabilities: (backend) => catalogService.getBackendCapabilities(backend),
      createSession: (backend, request) => sessionCommandService.createSession(backend, request),
      importSession: (route, title) => sessionCommandService.importSession(route, title),
      attachSession: (sessionId) => sessionCommandService.attachSession(sessionId),
      restoreSavedSession: (sessionId) => sessionCommandService.restoreSavedSession(sessionId),
      deleteSavedSession: (sessionId) => sessionCommandService.deleteSavedSession(sessionId),
      pruneSavedSessions: (keepLatest) => sessionCommandService.pruneSavedSessions(keepLatest),
      dispatchMuxCommand: (sessionId, command) =>
        sessionCommandService.dispatchMuxCommand(sessionId, command),
      openSubscription: (sessionId, spec) =>
        sessionCommandService.openSubscription(sessionId, spec),
      setActiveSession: (sessionId) => sessionCommandService.setActiveSession(sessionId),
      setActivePane: (paneId) => sessionCommandService.setActivePane(paneId),
      updateDraft: (paneId, value) => draftInputService.updateDraft(paneId, value),
      clearDraft: (paneId) => draftInputService.clearDraft(paneId),
      setTheme: (themeId) => themeResolutionService.setTheme(themeId),
      clearDiagnostics,
    },
    selectors,
    diagnostics,
  };

  function assertNotDisposed(): void {
    if (disposed) {
      throw new Error("workspace kernel has been disposed");
    }
  }
}

function resolveTransport(
  transport: WorkspaceTransportClient | WorkspaceTransportFactory,
): Promise<WorkspaceTransportClient> | WorkspaceTransportClient {
  if ("create" in transport) {
    return transport.create();
  }

  return transport;
}
