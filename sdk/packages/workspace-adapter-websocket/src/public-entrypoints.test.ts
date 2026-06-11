import { describe, expect, it } from "vitest";

import {
  WORKSPACE_WEBSOCKET_DIAGNOSTIC_CODES,
  createWorkspaceGatewayError,
  mapWorkspaceGatewayError,
} from "./index.js";
import type {
  WorkspaceGatewayControlClientMessage,
  WorkspaceGatewayControlRequestMap,
  WorkspaceGatewayControlServerResponse,
  WorkspaceGatewayStreamClientMessage,
  WorkspaceGatewayStreamServerMessage,
} from "./protocol.js";

type Assert<T extends true> = T;
type HasMethod<Map, Method extends PropertyKey> = Method extends keyof Map ? true : false;
type _HandshakeRequestIsPublic = Assert<
  HasMethod<WorkspaceGatewayControlRequestMap, "workspace_handshake">
>;
type _PaneHistoryRequestIsPublic = Assert<
  HasMethod<WorkspaceGatewayControlRequestMap, "workspace_pane_history">
>;

describe("workspace websocket protocol public subpath", () => {
  it("exposes websocket diagnostics mapping from the main public entrypoint", () => {
    const diagnostic = mapWorkspaceGatewayError({
      message: "daemon rejected request",
      code: "session_not_found",
    }, {
      phase: "response",
      plane: "control",
    });
    const error = createWorkspaceGatewayError({
      message: "daemon rejected request",
      code: "session_not_found",
    }, {
      phase: "response",
      plane: "control",
    });

    expect(WORKSPACE_WEBSOCKET_DIAGNOSTIC_CODES.gatewayError).toBe("websocket_gateway_error");
    expect(diagnostic).toMatchObject({
      workspaceErrorCode: "session_not_found",
      gatewayCode: "session_not_found",
      recoverable: true,
    });
    expect(error).toMatchObject({
      code: "session_not_found",
      recoverable: true,
    });
  });

  it("exposes gateway protocol types without adding runtime surface", async () => {
    const protocol = await import("./protocol.js");
    const controlRequest = {
      type: "request",
      requestId: "request-1",
      method: "workspace_handshake",
      payload: undefined,
    } satisfies WorkspaceGatewayControlClientMessage;
    const controlResponse = {
      type: "response",
      requestId: "request-1",
      method: "workspace_handshake",
      ok: false,
      error: {
        message: "daemon offline",
        code: "bootstrap_failed",
      },
    } satisfies WorkspaceGatewayControlServerResponse;
    const streamRequest = {
      type: "workspace_unsubscribe",
      subscriptionId: "subscription-1",
    } satisfies WorkspaceGatewayStreamClientMessage;
    const streamResponse = {
      type: "workspace_subscription_closed",
      subscriptionId: "subscription-1",
    } satisfies WorkspaceGatewayStreamServerMessage;

    expect(Object.keys(protocol)).toEqual([]);
    expect(controlRequest.method).toBe("workspace_handshake");
    expect(controlResponse.ok).toBe(false);
    expect(streamRequest.type).toBe("workspace_unsubscribe");
    expect(streamResponse.type).toBe("workspace_subscription_closed");
  });
});
