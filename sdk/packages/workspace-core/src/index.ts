export { createWorkspaceKernel } from "./kernel/create-workspace-kernel.js";

export type {
  CreateWorkspaceKernelOptions,
  WorkspaceCommands,
  WorkspaceDiagnostics,
  WorkspaceKernel,
  WorkspaceSelectors,
} from "./kernel/types.js";

export {
  createInitialWorkspaceSnapshot,
  type WorkspaceCatalogSnapshot,
  type WorkspaceConnectionSnapshot,
  type WorkspaceConnectionState,
  type WorkspaceDiagnosticRecord,
  type WorkspaceDiagnosticSeverity,
  type WorkspaceSelectionSnapshot,
  type WorkspaceSnapshot,
  type WorkspaceThemeSnapshot,
} from "./read-models/workspace-snapshot.js";
