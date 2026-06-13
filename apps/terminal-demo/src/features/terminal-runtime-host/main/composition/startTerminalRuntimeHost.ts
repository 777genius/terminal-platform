import { startWorkspaceGatewayNodeServer } from "@terminal-platform/workspace-gateway-node";
import { DaemonSupervisor } from "../infrastructure/DaemonSupervisor.js";
import { TerminalPlatformClientProvider } from "../infrastructure/TerminalPlatformClientProvider.js";
import {
  DEFAULT_TERMINAL_RUNTIME_SLUG,
  normalizeTerminalShellProgram,
} from "./shell-policy.js";
import type {
  WorkspaceGatewayFaultInjectionPort,
  WorkspaceGatewayNodeServerHandle,
  WorkspaceRuntimeClientPort,
} from "@terminal-platform/workspace-gateway-node";
import type { WorkspacePaneHistoryRequestOptions } from "@terminal-platform/workspace-contracts";

export {
  DEFAULT_TERMINAL_RUNTIME_SLUG,
  DEFAULT_TERMINAL_DEMO_MACOS_SHELL,
  DEFAULT_TERMINAL_DEMO_UNIX_SHELL,
  DEFAULT_TERMINAL_DEMO_WINDOWS_SHELL,
  resolveDemoDefaultWorkingDirectory,
  resolveDemoDefaultShellProgram,
} from "./shell-policy.js";
export type TerminalRuntimeGatewayFaultInjectionPort = WorkspaceGatewayFaultInjectionPort;

export interface TerminalRuntimeHostHandle {
  controlPlaneUrl: string;
  sessionStreamUrl: string;
  runtimeSlug: string;
  dispose(): Promise<void>;
}

export interface TerminalRuntimeInitialNativeSession {
  title?: string | null;
  program: string;
  args?: string[];
  cwd?: string | null;
}

interface TerminalRuntimeDaemonSupervisorPort {
  ensureRunning(): Promise<void>;
  dispose(): Promise<void>;
}

interface TerminalRuntimeGatewayServerHandle {
  controlPlaneUrl: string;
  sessionStreamUrl: string;
  runtimeSlug: string;
  dispose(): Promise<void>;
}

interface TerminalRuntimeHostDependencies {
  daemonSupervisor: TerminalRuntimeDaemonSupervisorPort;
  createClientProvider(runtimeSlug: string): TerminalPlatformClientProvider;
  startGateway(input: TerminalRuntimeGatewayStartInput): Promise<TerminalRuntimeGatewayServerHandle>;
}

interface TerminalRuntimeGatewayStartInput {
  runtimeSlug: string;
  runtime: WorkspaceRuntimeClientPort;
  faultInjection?: WorkspaceGatewayFaultInjectionPort | null;
}

export async function startTerminalRuntimeHost(options?: {
  runtimeSlug?: string;
  forceRestartReadyDaemon?: boolean;
  initialNativeSession?: TerminalRuntimeInitialNativeSession | null;
  sessionStorePath?: string | null;
  gatewayFaultInjection?: TerminalRuntimeGatewayFaultInjectionPort | null;
}): Promise<TerminalRuntimeHostHandle> {
  const runtimeSlug = options?.runtimeSlug ?? DEFAULT_TERMINAL_RUNTIME_SLUG;
  return startTerminalRuntimeHostWithDependencies(options, {
    daemonSupervisor: new DaemonSupervisor({
      runtimeSlug,
      forceRestartReadyDaemon: options?.forceRestartReadyDaemon ?? false,
      sessionStorePath: options?.sessionStorePath ?? null,
    }),
    createClientProvider: (slug) => new TerminalPlatformClientProvider(slug),
    startGateway: startTerminalRuntimeWorkspaceGateway,
  });
}

export async function startTerminalRuntimeHostWithDependencies(
  options: {
    runtimeSlug?: string;
    initialNativeSession?: TerminalRuntimeInitialNativeSession | null;
    gatewayFaultInjection?: TerminalRuntimeGatewayFaultInjectionPort | null;
  } | undefined,
  dependencies: TerminalRuntimeHostDependencies,
): Promise<TerminalRuntimeHostHandle> {
  const runtimeSlug = options?.runtimeSlug ?? DEFAULT_TERMINAL_RUNTIME_SLUG;
  let gatewayServer: TerminalRuntimeGatewayServerHandle | null = null;

  try {
    await dependencies.daemonSupervisor.ensureRunning();

    const clientProvider = dependencies.createClientProvider(runtimeSlug);
    if (options?.initialNativeSession) {
      await ensureInitialNativeSession(clientProvider, options.initialNativeSession);
    }

    const runtime = createWorkspaceRuntimeClientPort(clientProvider);
    gatewayServer = await dependencies.startGateway({
      runtimeSlug,
      runtime,
      faultInjection: options?.gatewayFaultInjection ?? null,
    });

    return {
      controlPlaneUrl: gatewayServer.controlPlaneUrl,
      sessionStreamUrl: gatewayServer.sessionStreamUrl,
      runtimeSlug: gatewayServer.runtimeSlug,
      dispose: async () => {
        await disposeTerminalRuntimeHostResources({
          daemonSupervisor: dependencies.daemonSupervisor,
          gatewayServer,
        });
      },
    };
  } catch (error) {
    await disposeTerminalRuntimeHostResources({
      daemonSupervisor: dependencies.daemonSupervisor,
      gatewayServer,
    }).catch(() => undefined);
    throw error;
  }
}

async function startTerminalRuntimeWorkspaceGateway(
  input: TerminalRuntimeGatewayStartInput,
): Promise<TerminalRuntimeGatewayServerHandle> {
  const handle = await startWorkspaceGatewayNodeServer({
    runtime: input.runtime,
    ...(input.faultInjection ? { faultInjection: input.faultInjection } : {}),
  });

  return toTerminalRuntimeGatewayServerHandle(input.runtimeSlug, handle);
}

function toTerminalRuntimeGatewayServerHandle(
  runtimeSlug: string,
  handle: WorkspaceGatewayNodeServerHandle,
): TerminalRuntimeGatewayServerHandle {
  return {
    controlPlaneUrl: handle.controlUrl,
    sessionStreamUrl: handle.streamUrl,
    runtimeSlug,
    dispose: () => handle.dispose(),
  };
}

function createWorkspaceRuntimeClientPort(
  clientProvider: TerminalPlatformClientProvider,
): WorkspaceRuntimeClientPort {
  return {
    async handshake() {
      const client = await clientProvider.getClient();
      return (await client.handshakeInfo()).handshake;
    },
    async listSessions() {
      const client = await clientProvider.getClient();
      return client.listSessions();
    },
    async listSavedSessions() {
      const client = await clientProvider.getClient();
      return client.listSavedSessions();
    },
    async listCommandHistory(sessionId, limit) {
      const client = await clientProvider.getClient();
      return client.commandHistory(sessionId ?? null, limit ?? null);
    },
    async getPaneHistory(sessionId, paneId, options?: WorkspacePaneHistoryRequestOptions) {
      const client = await clientProvider.getClient();
      return client.paneHistory(
        sessionId,
        paneId,
        toNullableSafeInteger(options?.fromEventSeq, "pane history fromEventSeq"),
        toNullableSafeInteger(options?.maxSegments, "pane history maxSegments"),
        toNullableSafeInteger(options?.maxBytes, "pane history maxBytes"),
      );
    },
    async discoverSessions(backend) {
      const client = await clientProvider.getClient();
      return client.discoverSessions(backend);
    },
    async getBackendCapabilities(backend) {
      const client = await clientProvider.getClient();
      return client.backendCapabilities(backend);
    },
    async createSession(backend, request) {
      if (backend !== "native") {
        throw new Error(`Unsupported backend ${backend}`);
      }

      const client = await clientProvider.getClient();
      return client.createNativeSession(request);
    },
    async importSession(route, title) {
      const client = await clientProvider.getClient();
      return client.importSession(route, title ?? null);
    },
    async getSavedSession(sessionId) {
      const client = await clientProvider.getClient();
      return client.savedSession(sessionId);
    },
    async deleteSavedSession(sessionId) {
      const client = await clientProvider.getClient();
      return client.deleteSavedSession(sessionId);
    },
    async pruneSavedSessions(keepLatest) {
      const client = await clientProvider.getClient();
      return client.pruneSavedSessions(keepLatest);
    },
    async restoreSavedSession(sessionId) {
      const client = await clientProvider.getClient();
      return client.restoreSavedSession(sessionId);
    },
    async attachSession(sessionId) {
      const client = await clientProvider.getClient();
      return client.attachSession(sessionId);
    },
    async getTopologySnapshot(sessionId) {
      const client = await clientProvider.getClient();
      return client.topologySnapshot(sessionId);
    },
    async getScreenSnapshot(sessionId, paneId) {
      const client = await clientProvider.getClient();
      return client.screenSnapshot(sessionId, paneId);
    },
    async getScreenDelta(sessionId, paneId, fromSequence) {
      if (fromSequence > BigInt(Number.MAX_SAFE_INTEGER)) {
        throw new Error("screen delta sequence exceeds the generated runtime client safe range");
      }

      const client = await clientProvider.getClient();
      return client.screenDelta(sessionId, paneId, Number(fromSequence));
    },
    async dispatchMuxCommand(sessionId, command) {
      const client = await clientProvider.getClient();
      return client.dispatchMuxCommand(sessionId, command);
    },
    async openSubscription(sessionId, spec) {
      const client = await clientProvider.getClient();
      const subscription = await client.openSubscription(sessionId, spec);
      return {
        meta: () => ({
          subscription_id: subscription.subscriptionId,
        }),
        nextEvent: () => subscription.nextEvent(),
        close: () => subscription.close(),
      };
    },
    async close() {
      // The generated TerminalNodeClient does not own the daemon lifecycle.
    },
  };
}

function toNullableSafeInteger(
  value: bigint | number | null | undefined,
  label: string,
): number | null {
  if (value == null) {
    return null;
  }

  if (typeof value === "bigint") {
    if (value > BigInt(Number.MAX_SAFE_INTEGER) || value < BigInt(Number.MIN_SAFE_INTEGER)) {
      throw new Error(`${label} exceeds native client safe integer range`);
    }
    return Number(value);
  }

  if (!Number.isSafeInteger(value)) {
    throw new Error(`${label} must be a safe integer`);
  }

  return value;
}

export async function disposeTerminalRuntimeHostResources(resources: {
  daemonSupervisor?: TerminalRuntimeDaemonSupervisorPort | null;
  gatewayServer?: TerminalRuntimeGatewayServerHandle | null;
}): Promise<void> {
  const disposals = [
    resources.gatewayServer?.dispose(),
    resources.daemonSupervisor?.dispose(),
  ].filter((disposal): disposal is Promise<void> => Boolean(disposal));

  const results = await Promise.allSettled(disposals);
  const failures = results.filter((result): result is PromiseRejectedResult => result.status === "rejected");
  const [firstFailure] = failures;
  if (failures.length === 1 && firstFailure) {
    throw firstFailure.reason;
  }

  if (failures.length > 1) {
    throw new AggregateError(
      failures.map((failure) => failure.reason),
      "Failed to dispose terminal runtime host resources",
    );
  }
}

async function ensureInitialNativeSession(
  clientProvider: TerminalPlatformClientProvider,
  session: TerminalRuntimeInitialNativeSession,
): Promise<void> {
  const program = normalizeTerminalShellProgram(session.program);
  if (!program) {
    return;
  }

  const client = await clientProvider.getClient();
  const existingSessions = await client.listSessions();
  if (existingSessions.length > 0) {
    return;
  }

  await client.createNativeSession({
    title: normalizeOptionalString(session.title),
    launch: {
      program,
      args: session.args ?? [],
      cwd: normalizeOptionalString(session.cwd),
    },
  });
}

function normalizeOptionalString(value: string | null | undefined): string | null {
  const normalized = value?.trim();
  return normalized || null;
}
