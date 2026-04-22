import {
  TerminalWorkspaceControlService,
  TerminalWorkspaceSessionStreamService,
} from "../../core/application/index.js";
import { TerminalWorkspaceGatewayServer } from "../adapters/input/TerminalWorkspaceGatewayServer.js";
import { TerminalPlatformControlRuntimeAdapter } from "../adapters/output/TerminalPlatformControlRuntimeAdapter.js";
import { TerminalPlatformSessionStateRuntimeAdapter } from "../adapters/output/TerminalPlatformSessionStateRuntimeAdapter.js";
import { DaemonSupervisor } from "../infrastructure/DaemonSupervisor.js";
import { TerminalPlatformClientProvider } from "../infrastructure/TerminalPlatformClientProvider.js";

export const DEFAULT_TERMINAL_WORKSPACE_RUNTIME_SLUG = "terminal-demo";

export interface TerminalWorkspaceHostHandle {
  controlPlaneUrl: string;
  sessionStreamUrl: string;
  runtimeSlug: string;
  dispose(): Promise<void>;
}

export async function startTerminalWorkspaceHost(options?: {
  runtimeSlug?: string;
}): Promise<TerminalWorkspaceHostHandle> {
  const runtimeSlug = options?.runtimeSlug ?? DEFAULT_TERMINAL_WORKSPACE_RUNTIME_SLUG;
  const daemonSupervisor = new DaemonSupervisor({ runtimeSlug });
  await daemonSupervisor.ensureRunning();

  const clientProvider = new TerminalPlatformClientProvider(runtimeSlug);
  const controlRuntimeAdapter = new TerminalPlatformControlRuntimeAdapter(clientProvider);
  const sessionStateRuntimeAdapter = new TerminalPlatformSessionStateRuntimeAdapter(clientProvider);
  const controlService = new TerminalWorkspaceControlService(controlRuntimeAdapter);
  const sessionStreamService = new TerminalWorkspaceSessionStreamService(sessionStateRuntimeAdapter);
  const gatewayServer = await TerminalWorkspaceGatewayServer.start({
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
