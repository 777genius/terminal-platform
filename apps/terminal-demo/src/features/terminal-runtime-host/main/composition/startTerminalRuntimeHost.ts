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

export interface TerminalRuntimeHostHandle {
  controlPlaneUrl: string;
  sessionStreamUrl: string;
  runtimeSlug: string;
  dispose(): Promise<void>;
}

export async function startTerminalRuntimeHost(options?: {
  runtimeSlug?: string;
  forceRestartReadyDaemon?: boolean;
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
