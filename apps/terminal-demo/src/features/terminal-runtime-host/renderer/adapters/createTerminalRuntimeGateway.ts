import type {
  TerminalWorkspaceControlGatewayPort,
  TerminalWorkspaceSessionStateStreamPort,
} from "../../core/application/index.js";
import { WebSocketTerminalRuntimeControlPlane } from "./WebSocketTerminalRuntimeControlPlane.js";
import { WebSocketTerminalRuntimeSessionStateStream } from "./WebSocketTerminalRuntimeSessionStateStream.js";

export interface TerminalRuntimeGatewayPlanes {
  controlPlane: TerminalWorkspaceControlGatewayPort;
  sessionStatePlane: TerminalWorkspaceSessionStateStreamPort;
  controlPlaneUrl: string;
  sessionStreamUrl: string;
  dispose(): void;
}

export function createTerminalRuntimeGateway(config: {
  controlPlaneUrl: string;
  sessionStreamUrl: string;
}): TerminalRuntimeGatewayPlanes {
  const controlPlane = new WebSocketTerminalRuntimeControlPlane(config.controlPlaneUrl);
  const sessionStreamUrl = config.sessionStreamUrl;
  const sessionStatePlane = new WebSocketTerminalRuntimeSessionStateStream(sessionStreamUrl);

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
