import { useSyncExternalStore } from "react";

import type { WorkspaceKernel, WorkspaceSnapshot } from "@terminal-platform/workspace-core";

export function useWorkspaceSnapshot(kernel: WorkspaceKernel): WorkspaceSnapshot {
  return useSyncExternalStore(kernel.subscribe, kernel.getSnapshot, kernel.getSnapshot);
}
