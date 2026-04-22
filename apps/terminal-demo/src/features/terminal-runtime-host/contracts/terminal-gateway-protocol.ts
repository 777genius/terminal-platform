import type {
  TerminalBackendCapabilitiesInfo,
  TerminalBackendKind,
  TerminalCreateNativeSessionInput,
  TerminalDeleteSavedSessionResponse,
  TerminalDiscoveredSession,
  TerminalHandshakeInfo,
  TerminalImportSessionInput,
  TerminalMuxCommand,
  TerminalMuxCommandResult,
  TerminalSavedSessionSummary,
  TerminalSessionState,
  TerminalSessionSummary,
} from "@features/terminal-workspace-kernel/contracts";

export type TerminalGatewayControlMethod =
  | "handshake_info"
  | "list_sessions"
  | "list_saved_sessions"
  | "discover_sessions"
  | "backend_capabilities"
  | "create_native_session"
  | "import_session"
  | "restore_saved_session"
  | "delete_saved_session"
  | "dispatch_mux_command";

export interface TerminalGatewayControlRequestMap {
  handshake_info: {
    payload: undefined;
    response: TerminalHandshakeInfo;
  };
  list_sessions: {
    payload: undefined;
    response: TerminalSessionSummary[];
  };
  list_saved_sessions: {
    payload: undefined;
    response: TerminalSavedSessionSummary[];
  };
  discover_sessions: {
    payload: { backend: TerminalBackendKind };
    response: TerminalDiscoveredSession[];
  };
  backend_capabilities: {
    payload: { backend: TerminalBackendKind };
    response: TerminalBackendCapabilitiesInfo;
  };
  create_native_session: {
    payload: TerminalCreateNativeSessionInput;
    response: TerminalSessionSummary;
  };
  import_session: {
    payload: TerminalImportSessionInput;
    response: TerminalSessionSummary;
  };
  restore_saved_session: {
    payload: { sessionId: string };
    response: TerminalSessionSummary;
  };
  delete_saved_session: {
    payload: { sessionId: string };
    response: TerminalDeleteSavedSessionResponse;
  };
  dispatch_mux_command: {
    payload: {
      sessionId: string;
      command: TerminalMuxCommand;
    };
    response: TerminalMuxCommandResult;
  };
}

export type TerminalGatewayControlClientMessage = {
  [Method in keyof TerminalGatewayControlRequestMap]: {
    type: "request";
    requestId: string;
    method: Method;
    payload: TerminalGatewayControlRequestMap[Method]["payload"];
  };
}[keyof TerminalGatewayControlRequestMap];

export interface TerminalGatewayErrorEnvelope {
  message: string;
  code?: string;
}

export type TerminalGatewayControlServerResponse = {
  [Method in keyof TerminalGatewayControlRequestMap]: {
    type: "response";
    requestId: string;
    method: Method;
    ok: true;
    result: TerminalGatewayControlRequestMap[Method]["response"];
  };
}[keyof TerminalGatewayControlRequestMap] | {
  type: "response";
  requestId: string;
  method: TerminalGatewayControlMethod;
  ok: false;
  error: TerminalGatewayErrorEnvelope;
};

export type TerminalGatewayStreamClientMessage =
  | {
      type: "stream_subscribe_session_state";
      subscriptionId: string;
      sessionId: string;
    }
  | {
      type: "stream_unsubscribe_session_state";
      subscriptionId: string;
      sessionId: string;
    };

export type TerminalGatewayStreamServerMessage =
  | {
      type: "stream_subscription_ack";
      subscriptionId: string;
      sessionId: string;
    }
  | {
      type: "stream_subscription_rejected";
      subscriptionId: string;
      sessionId: string;
      error: TerminalGatewayErrorEnvelope;
    }
  | {
      type: "session_state";
      subscriptionId: string;
      sessionId: string;
      state: TerminalSessionState;
    }
  | {
      type: "subscription_error";
      subscriptionId: string;
      sessionId: string;
      error: TerminalGatewayErrorEnvelope;
    }
  | {
      type: "subscription_closed";
      subscriptionId: string;
      sessionId: string;
    };
