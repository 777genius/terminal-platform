import {
  TerminalRuntimeControlService,
  TerminalRuntimeSessionStreamService,
} from "../../core/application/index.js";
import { TerminalRuntimeGatewayServer } from "../adapters/input/TerminalRuntimeGatewayServer.js";
import { TerminalPlatformControlRuntimeAdapter } from "../adapters/output/TerminalPlatformControlRuntimeAdapter.js";
import { TerminalPlatformSessionStateRuntimeAdapter } from "../adapters/output/TerminalPlatformSessionStateRuntimeAdapter.js";
import { DaemonSupervisor } from "../infrastructure/DaemonSupervisor.js";
import { TerminalPlatformClientProvider } from "../infrastructure/TerminalPlatformClientProvider.js";

export const DEFAULT_TERMINAL_RUNTIME_SLUG = "terminal-demo";
export const DEFAULT_TERMINAL_DEMO_UNIX_SHELL = "bash";
export const DEFAULT_TERMINAL_DEMO_MACOS_SHELL = "zsh";
export const DEFAULT_TERMINAL_DEMO_WINDOWS_SHELL = "pwsh.exe";

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

export async function startTerminalRuntimeHost(options?: {
  runtimeSlug?: string;
  forceRestartReadyDaemon?: boolean;
  initialNativeSession?: TerminalRuntimeInitialNativeSession | null;
  sessionStorePath?: string | null;
}): Promise<TerminalRuntimeHostHandle> {
  const runtimeSlug = options?.runtimeSlug ?? DEFAULT_TERMINAL_RUNTIME_SLUG;
  const daemonSupervisor = new DaemonSupervisor({
    runtimeSlug,
    forceRestartReadyDaemon: options?.forceRestartReadyDaemon ?? false,
    sessionStorePath: options?.sessionStorePath ?? null,
  });
  await daemonSupervisor.ensureRunning();

  const clientProvider = new TerminalPlatformClientProvider(runtimeSlug);
  if (options?.initialNativeSession) {
    await ensureInitialNativeSession(clientProvider, options.initialNativeSession);
  }

  const controlRuntimeAdapter = new TerminalPlatformControlRuntimeAdapter(clientProvider);
  const sessionStateRuntimeAdapter = new TerminalPlatformSessionStateRuntimeAdapter(clientProvider);
  const controlService = new TerminalRuntimeControlService(controlRuntimeAdapter);
  const sessionStreamService = new TerminalRuntimeSessionStreamService(sessionStateRuntimeAdapter);
  const gatewayServer = await TerminalRuntimeGatewayServer.start({
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
      await gatewayServer.dispose();
      await daemonSupervisor.dispose();
    },
  };
}

async function ensureInitialNativeSession(
  clientProvider: TerminalPlatformClientProvider,
  session: TerminalRuntimeInitialNativeSession,
): Promise<void> {
  const program = normalizeShellProgram(session.program);
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

export function resolveDemoDefaultShellProgram(options: {
  env?: Readonly<Record<string, string | undefined>>;
  platform?: NodeJS.Platform;
} = {}): string {
  const env = options.env ?? process.env;
  const platform = options.platform ?? process.platform;
  const explicitProgram = normalizeShellProgram(env.TERMINAL_DEMO_DEFAULT_SHELL);
  if (explicitProgram) {
    return explicitProgram;
  }

  if (platform === "win32") {
    return normalizeShellProgram(env.ComSpec)
      ?? normalizeShellProgram(env.COMSPEC)
      ?? DEFAULT_TERMINAL_DEMO_WINDOWS_SHELL;
  }

  return normalizeShellProgram(env.SHELL)
    ?? (platform === "darwin" ? DEFAULT_TERMINAL_DEMO_MACOS_SHELL : DEFAULT_TERMINAL_DEMO_UNIX_SHELL);
}

function normalizeShellProgram(value: string | null | undefined): string | null {
  const normalized = value?.trim();
  return normalized || null;
}

function normalizeOptionalString(value: string | null | undefined): string | null {
  const normalized = value?.trim();
  return normalized || null;
}
