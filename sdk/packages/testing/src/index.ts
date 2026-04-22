import {
  createDefaultMemoryWorkspaceFixture,
  createMemoryWorkspaceTransport,
  type CreateMemoryWorkspaceTransportOptions,
} from "@terminal-platform/workspace-adapter-memory";
import { createWorkspaceKernel, type WorkspaceKernel } from "@terminal-platform/workspace-core";

export interface WorkspaceTestHarness {
  kernel: WorkspaceKernel;
  dispose(): Promise<void>;
}

export function createWorkspaceTestHarness(
  options: CreateMemoryWorkspaceTransportOptions = {},
): WorkspaceTestHarness {
  const transport = createMemoryWorkspaceTransport(options);
  const kernel = createWorkspaceKernel({ transport });

  return {
    kernel,
    dispose() {
      return kernel.dispose();
    },
  };
}

export { createDefaultMemoryWorkspaceFixture };
