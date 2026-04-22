import type {
  BackendKind,
  CreateSessionRequest,
  MuxCommand,
  PaneId,
  SessionId,
  SessionRoute,
  SubscriptionSpec,
} from "@terminal-platform/runtime-types";

export interface CreateWorkspaceSessionInput {
  backend: BackendKind;
  request: CreateSessionRequest;
}

export interface ImportWorkspaceSessionInput {
  route: SessionRoute;
  title?: string | null;
}

export interface AttachWorkspaceSessionInput {
  sessionId: SessionId;
}

export interface OpenWorkspaceSubscriptionInput {
  sessionId: SessionId;
  spec: SubscriptionSpec;
}

export interface DispatchWorkspaceMuxCommandInput {
  sessionId: SessionId;
  command: MuxCommand;
}

export interface RequestWorkspaceScreenInput {
  sessionId: SessionId;
  paneId: PaneId;
}

export interface RequestWorkspaceScreenDeltaInput extends RequestWorkspaceScreenInput {
  fromSequence: bigint;
}

export interface DeleteSavedWorkspaceSessionInput {
  sessionId: SessionId;
}

export interface RestoreSavedWorkspaceSessionInput {
  sessionId: SessionId;
}

export interface PruneSavedWorkspaceSessionsInput {
  keepLatest: number;
}
