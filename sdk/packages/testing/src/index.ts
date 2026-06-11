import {
  createDefaultMemoryWorkspaceFixture,
  createMemoryWorkspaceTransport,
  type CreateMemoryWorkspaceTransportOptions,
} from "@terminal-platform/workspace-adapter-memory";
import type { WorkspaceTransportClient } from "@terminal-platform/workspace-contracts";
import type { WorkspaceKernel } from "@terminal-platform/workspace-core";
import { createWorkspaceHost, type WorkspaceHost } from "@terminal-platform/workspace-core/bootstrap";

export {
  TERMINAL_PLATFORM_PACKED_CONSUMER_SMOKE_SPEC,
  assertTerminalPlatformPackedConsumerSmoke,
  runTerminalPlatformPackedConsumerSmoke,
  type RunTerminalPlatformPackedConsumerSmokeOptions,
  type TerminalPlatformPackageImporter,
  type TerminalPlatformPackedConsumerSmokeEntry,
  type TerminalPlatformPackedConsumerSmokeFailure,
  type TerminalPlatformPackedConsumerSmokeResult,
} from "./packed-consumer-smoke.js";

export interface WorkspaceTestHarness {
  host: WorkspaceHost;
  kernel: WorkspaceKernel;
  transport: WorkspaceTransportClient;
  bootstrap(): Promise<WorkspaceKernel>;
  dispose(): Promise<void>;
}

export interface CreateWorkspaceTestHarnessOptions extends CreateMemoryWorkspaceTransportOptions {
  autoBootstrap?: boolean;
}

export function createWorkspaceTestHarness(
  options: CreateWorkspaceTestHarnessOptions = {},
): WorkspaceTestHarness {
  const { autoBootstrap = false, ...transportOptions } = options;
  const transport = createMemoryWorkspaceTransport(transportOptions);
  const host = createWorkspaceHost({
    autoBootstrap,
    transport,
  });

  return {
    host,
    kernel: host.kernel,
    transport,
    bootstrap: () => host.bootstrap(),
    dispose() {
      return host.dispose();
    },
  };
}

export { createDefaultMemoryWorkspaceFixture };
