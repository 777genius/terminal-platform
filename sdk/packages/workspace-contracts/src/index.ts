export {
  WORKSPACE_CONTRACTS_COMPATIBILITY,
  WORKSPACE_CONTRACTS_SCHEMA_VERSION,
} from "./compat/workspace-contracts-compat.js";
export type { WorkspaceContractsCompatibility } from "./compat/workspace-contracts-compat.js";

export type {
  AttachWorkspaceSessionInput,
  CreateWorkspaceSessionInput,
  DeleteSavedWorkspaceSessionInput,
  DispatchWorkspaceMuxCommandInput,
  ImportWorkspaceSessionInput,
  OpenWorkspaceSubscriptionInput,
  PruneSavedWorkspaceSessionsInput,
  RequestWorkspaceScreenDeltaInput,
  RequestWorkspaceScreenInput,
  RestoreSavedWorkspaceSessionInput,
} from "./commands/workspace-commands.js";

export { WorkspaceError, toWorkspaceError } from "./errors/workspace-error.js";
export type { WorkspaceErrorCode, WorkspaceErrorShape } from "./errors/workspace-error.js";

export type { WorkspaceObservation } from "./observations/workspace-observations.js";

export type {
  WorkspaceSubscription,
  WorkspaceTransportClient,
  WorkspaceTransportFactory,
} from "./ports/workspace-transport.js";
