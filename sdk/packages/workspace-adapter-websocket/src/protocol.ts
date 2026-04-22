import type {
  BackendCapabilitiesInfo,
  BackendKind,
  CreateSessionRequest,
  DeleteSavedSessionResult,
  DiscoveredSession,
  Handshake,
  MuxCommand,
  MuxCommandResult,
  PaneId,
  PruneSavedSessionsResult,
  RestoredSession,
  SavedSessionRecord,
  SavedSessionSummary,
  ScreenDelta,
  ScreenSnapshot,
  SessionId,
  SessionRoute,
  SessionSummary,
  SubscriptionEvent,
  SubscriptionMeta,
  SubscriptionSpec,
  TopologySnapshot,
  AttachedSession,
} from "@terminal-platform/runtime-types";

export type WorkspaceGatewayControlMethod =
  | "workspace_handshake"
  | "workspace_list_sessions"
  | "workspace_list_saved_sessions"
  | "workspace_discover_sessions"
  | "workspace_backend_capabilities"
  | "workspace_create_session"
  | "workspace_import_session"
  | "workspace_saved_session"
  | "workspace_prune_saved_sessions"
  | "workspace_restore_saved_session"
  | "workspace_delete_saved_session"
  | "workspace_attach_session"
  | "workspace_topology_snapshot"
  | "workspace_screen_snapshot"
  | "workspace_screen_delta"
  | "workspace_dispatch_mux_command";

export interface WorkspaceGatewayControlRequestMap {
  workspace_handshake: {
    payload: undefined;
    response: Handshake;
  };
  workspace_list_sessions: {
    payload: undefined;
    response: SessionSummary[];
  };
  workspace_list_saved_sessions: {
    payload: undefined;
    response: SavedSessionSummary[];
  };
  workspace_discover_sessions: {
    payload: { backend: BackendKind };
    response: DiscoveredSession[];
  };
  workspace_backend_capabilities: {
    payload: { backend: BackendKind };
    response: BackendCapabilitiesInfo;
  };
  workspace_create_session: {
    payload: {
      backend: BackendKind;
      request: CreateSessionRequest;
    };
    response: SessionSummary;
  };
  workspace_import_session: {
    payload: {
      route: SessionRoute;
      title?: string | null;
    };
    response: SessionSummary;
  };
  workspace_saved_session: {
    payload: { sessionId: SessionId };
    response: SavedSessionRecord;
  };
  workspace_prune_saved_sessions: {
    payload: { keepLatest: number };
    response: PruneSavedSessionsResult;
  };
  workspace_restore_saved_session: {
    payload: { sessionId: SessionId };
    response: RestoredSession;
  };
  workspace_delete_saved_session: {
    payload: { sessionId: SessionId };
    response: DeleteSavedSessionResult;
  };
  workspace_attach_session: {
    payload: { sessionId: SessionId };
    response: AttachedSession;
  };
  workspace_topology_snapshot: {
    payload: { sessionId: SessionId };
    response: TopologySnapshot;
  };
  workspace_screen_snapshot: {
    payload: {
      sessionId: SessionId;
      paneId: PaneId;
    };
    response: ScreenSnapshot;
  };
  workspace_screen_delta: {
    payload: {
      sessionId: SessionId;
      paneId: PaneId;
      fromSequence: bigint;
    };
    response: ScreenDelta;
  };
  workspace_dispatch_mux_command: {
    payload: {
      sessionId: SessionId;
      command: MuxCommand;
    };
    response: MuxCommandResult;
  };
}

export type WorkspaceGatewayErrorEnvelope = {
  message: string;
  code?: string;
};

export type WorkspaceGatewayControlClientMessage = {
  [Method in keyof WorkspaceGatewayControlRequestMap]: {
    type: "request";
    requestId: string;
    method: Method;
    payload: WorkspaceGatewayControlRequestMap[Method]["payload"];
  };
}[keyof WorkspaceGatewayControlRequestMap];

export type WorkspaceGatewayControlServerResponse =
  | {
      [Method in keyof WorkspaceGatewayControlRequestMap]: {
        type: "response";
        requestId: string;
        method: Method;
        ok: true;
        result: WorkspaceGatewayControlRequestMap[Method]["response"];
      };
    }[keyof WorkspaceGatewayControlRequestMap]
  | {
      type: "response";
      requestId: string;
      method: WorkspaceGatewayControlMethod;
      ok: false;
      error: WorkspaceGatewayErrorEnvelope;
    };

export type WorkspaceGatewayStreamClientMessage =
  | {
      type: "workspace_subscribe";
      subscriptionId: string;
      sessionId: SessionId;
      spec: SubscriptionSpec;
    }
  | {
      type: "workspace_unsubscribe";
      subscriptionId: string;
    };

export type WorkspaceGatewayStreamServerMessage =
  | {
      type: "workspace_subscription_ack";
      subscriptionId: string;
      meta: SubscriptionMeta;
    }
  | {
      type: "workspace_subscription_rejected";
      subscriptionId: string;
      error: WorkspaceGatewayErrorEnvelope;
    }
  | {
      type: "workspace_subscription_event";
      subscriptionId: string;
      event: SubscriptionEvent;
    }
  | {
      type: "workspace_subscription_error";
      subscriptionId: string;
      error: WorkspaceGatewayErrorEnvelope;
    }
  | {
      type: "workspace_subscription_closed";
      subscriptionId: string;
    };
