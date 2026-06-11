import { WorkspaceError } from "@terminal-platform/workspace-contracts";

import { createWorkspaceKernel } from "./kernel/create-workspace-kernel.js";

import type { CreateWorkspaceKernelOptions, WorkspaceKernel } from "./kernel/types.js";

export interface CreateWorkspaceHostOptions extends CreateWorkspaceKernelOptions {
  autoBootstrap?: boolean;
}

export interface WorkspaceHost {
  readonly kernel: WorkspaceKernel;
  bootstrap(): Promise<WorkspaceKernel>;
  dispose(): Promise<void>;
}

export function createWorkspaceHost(options: CreateWorkspaceHostOptions): WorkspaceHost {
  const { autoBootstrap = false, ...kernelOptions } = options;
  const kernel = createWorkspaceKernel(kernelOptions);
  let bootstrapPromise: Promise<WorkspaceKernel> | null = null;
  let disposed = false;

  function bootstrap(): Promise<WorkspaceKernel> {
    if (disposed) {
      return Promise.reject(new WorkspaceError({
        code: "disposed",
        message: "workspace host has been disposed",
        recoverable: false,
      }));
    }

    if (!bootstrapPromise) {
      bootstrapPromise = kernel.bootstrap()
        .then(() => kernel)
        .catch((error) => {
          bootstrapPromise = null;
          throw error;
        });
    }

    return bootstrapPromise;
  }

  async function dispose(): Promise<void> {
    if (disposed) {
      return;
    }

    disposed = true;
    await kernel.dispose();
  }

  const host = {
    kernel,
    bootstrap,
    dispose,
  };

  if (autoBootstrap) {
    void bootstrap().catch(() => undefined);
  }

  return host;
}
