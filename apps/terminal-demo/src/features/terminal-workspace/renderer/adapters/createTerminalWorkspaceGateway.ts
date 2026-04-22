import type {
  TerminalWorkspaceControlGatewayPort,
  TerminalWorkspaceSessionStateStreamPort,
} from "../../core/application/index.js";
import { WebSocketTerminalWorkspaceControlPlane } from "./WebSocketTerminalWorkspaceControlPlane.js";
import { WebSocketTerminalWorkspaceSessionStateStream } from "./WebSocketTerminalWorkspaceSessionStateStream.js";

export interface TerminalWorkspaceGatewayPlanes {
  controlPlane: TerminalWorkspaceControlGatewayPort;
  sessionStatePlane: TerminalWorkspaceSessionStateStreamPort;
  controlPlaneUrl: string;
  sessionStreamUrl: string;
  dispose(): void;
}

export function createTerminalWorkspaceGateway(config: {
  controlPlaneUrl: string;
  sessionStreamUrl: string;
}): TerminalWorkspaceGatewayPlanes {
  const controlPlane = new WebSocketTerminalWorkspaceControlPlane(config.controlPlaneUrl);
  const sessionStreamUrl = config.sessionStreamUrl;
  const sessionStatePlane = new WebSocketTerminalWorkspaceSessionStateStream(sessionStreamUrl);

  return {
    controlPlane,
    sessionStatePlane,
    controlPlaneUrl: config.controlPlaneUrl,
    sessionStreamUrl,
    dispose() {
      sessionStatePlane.dispose();
      controlPlane.dispose();
    },
  };
}
