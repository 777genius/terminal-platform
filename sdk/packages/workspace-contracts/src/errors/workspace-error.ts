export type WorkspaceErrorCode =
  | "bootstrap_failed"
  | "transport_failed"
  | "protocol_error"
  | "session_not_found"
  | "pane_not_found"
  | "subscription_failed"
  | "unsupported_capability"
  | "disposed";

export interface WorkspaceErrorShape {
  code: WorkspaceErrorCode;
  message: string;
  recoverable: boolean;
  cause?: unknown;
}

export class WorkspaceError extends Error implements WorkspaceErrorShape {
  readonly code: WorkspaceErrorCode;
  readonly recoverable: boolean;
  override readonly cause?: unknown;

  constructor(input: WorkspaceErrorShape) {
    super(input.message);
    this.name = "WorkspaceError";
    this.code = input.code;
    this.recoverable = input.recoverable;
    this.cause = input.cause;
  }
}

export function toWorkspaceError(
  error: unknown,
  fallback: Omit<WorkspaceErrorShape, "cause">,
): WorkspaceError {
  if (error instanceof WorkspaceError) {
    return error;
  }

  if (error instanceof Error) {
    return new WorkspaceError({
      ...fallback,
      message: error.message,
      cause: error,
    });
  }

  return new WorkspaceError({
    ...fallback,
    cause: error,
  });
}
