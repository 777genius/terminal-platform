import {
  TerminalRuntimeControlService,
  TerminalRuntimeSessionStreamService,
} from "../../core/application/index.js";
import { TerminalRuntimeGatewayServer } from "../adapters/input/TerminalRuntimeGatewayServer.js";
import { TerminalPlatformControlRuntimeAdapter } from "../adapters/output/TerminalPlatformControlRuntimeAdapter.js";
import { TerminalPlatformSessionStateRuntimeAdapter } from "../adapters/output/TerminalPlatformSessionStateRuntimeAdapter.js";
import { DaemonSupervisor } from "../infrastructure/DaemonSupervisor.js";
import { TerminalPlatformClientProvider } from "../infrastructure/TerminalPlatformClientProvider.js";
import {
  DEFAULT_TERMINAL_RUNTIME_SLUG,
  normalizeTerminalShellProgram,
} from "./shell-policy.js";

export {
  DEFAULT_TERMINAL_RUNTIME_SLUG,
  DEFAULT_TERMINAL_DEMO_MACOS_SHELL,
  DEFAULT_TERMINAL_DEMO_UNIX_SHELL,
  DEFAULT_TERMINAL_DEMO_WINDOWS_SHELL,
  resolveDemoDefaultWorkingDirectory,
  resolveDemoDefaultShellProgram,
} from "./shell-policy.js";

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
  startGateway: typeof TerminalRuntimeGatewayServer.start;
}

export async function startTerminalRuntimeHost(options?: {
  runtimeSlug?: string;
  forceRestartReadyDaemon?: boolean;
  initialNativeSession?: TerminalRuntimeInitialNativeSession | null;
  sessionStorePath?: string | null;
}): Promise<TerminalRuntimeHostHandle> {
  const runtimeSlug = options?.runtimeSlug ?? DEFAULT_TERMINAL_RUNTIME_SLUG;
  return startTerminalRuntimeHostWithDependencies(options, {
    daemonSupervisor: new DaemonSupervisor({
      runtimeSlug,
      forceRestartReadyDaemon: options?.forceRestartReadyDaemon ?? false,
      sessionStorePath: options?.sessionStorePath ?? null,
    }),
    createClientProvider: (slug) => new TerminalPlatformClientProvider(slug),
    startGateway: (input) => TerminalRuntimeGatewayServer.start(input),
  });
}

export async function startTerminalRuntimeHostWithDependencies(
  options: {
    runtimeSlug?: string;
    initialNativeSession?: TerminalRuntimeInitialNativeSession | null;
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

    const controlRuntimeAdapter = new TerminalPlatformControlRuntimeAdapter(clientProvider);
    const sessionStateRuntimeAdapter = new TerminalPlatformSessionStateRuntimeAdapter(clientProvider);
    const controlService = new TerminalRuntimeControlService(controlRuntimeAdapter);
    const sessionStreamService = new TerminalRuntimeSessionStreamService(sessionStateRuntimeAdapter);
    gatewayServer = await dependencies.startGateway({
      runtimeSlug,
      controlService,
      sessionStreamService,
      clientProvider,
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
