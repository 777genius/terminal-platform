export type { NodeAttachedSession } from "./bindings/NodeAttachedSession";
export type { NodeBackendKind } from "./bindings/NodeBackendKind";
export type { NodeBindingVersion } from "./bindings/NodeBindingVersion";
export type { NodeCreateSessionRequest } from "./bindings/NodeCreateSessionRequest";
export type { NodeDaemonCapabilities } from "./bindings/NodeDaemonCapabilities";
export type { NodeDaemonPhase } from "./bindings/NodeDaemonPhase";
export type { NodeExternalSessionRef } from "./bindings/NodeExternalSessionRef";
export type { NodeHandshake } from "./bindings/NodeHandshake";
export type { NodeHandshakeAssessment } from "./bindings/NodeHandshakeAssessment";
export type { NodeHandshakeAssessmentStatus } from "./bindings/NodeHandshakeAssessmentStatus";
export type { NodeHandshakeInfo } from "./bindings/NodeHandshakeInfo";
export type { NodePaneSplit } from "./bindings/NodePaneSplit";
export type { NodePaneTreeNode } from "./bindings/NodePaneTreeNode";
export type { NodeProjectionSource } from "./bindings/NodeProjectionSource";
export type { NodeProtocolCompatibility } from "./bindings/NodeProtocolCompatibility";
export type { NodeProtocolCompatibilityStatus } from "./bindings/NodeProtocolCompatibilityStatus";
export type { NodeProtocolVersion } from "./bindings/NodeProtocolVersion";
export type { NodeRouteAuthority } from "./bindings/NodeRouteAuthority";
export type { NodeScreenCursor } from "./bindings/NodeScreenCursor";
export type { NodeScreenLine } from "./bindings/NodeScreenLine";
export type { NodeScreenSnapshot } from "./bindings/NodeScreenSnapshot";
export type { NodeScreenSurface } from "./bindings/NodeScreenSurface";
export type { NodeSessionRoute } from "./bindings/NodeSessionRoute";
export type { NodeSessionSummary } from "./bindings/NodeSessionSummary";
export type { NodeShellLaunchSpec } from "./bindings/NodeShellLaunchSpec";
export type { NodeSplitDirection } from "./bindings/NodeSplitDirection";
export type { NodeTabSnapshot } from "./bindings/NodeTabSnapshot";
export type { NodeTopologySnapshot } from "./bindings/NodeTopologySnapshot";

import type { NodeAttachedSession } from "./bindings/NodeAttachedSession";
import type { NodeBindingVersion } from "./bindings/NodeBindingVersion";
import type { NodeCreateSessionRequest } from "./bindings/NodeCreateSessionRequest";
import type { NodeHandshakeInfo } from "./bindings/NodeHandshakeInfo";
import type { NodeScreenSnapshot } from "./bindings/NodeScreenSnapshot";
import type { NodeSessionSummary } from "./bindings/NodeSessionSummary";
import type { NodeTopologySnapshot } from "./bindings/NodeTopologySnapshot";

export interface NativeBindingLoadOptions {
  addonPath?: string | undefined;
}

export interface NativeTerminalNodeClientHandle {
  readonly address: string;
  bindingVersion(): NodeBindingVersion;
  handshakeInfo(): Promise<NodeHandshakeInfo>;
  listSessions(): Promise<NodeSessionSummary[]>;
  createNativeSession(request: NodeCreateSessionRequest): Promise<NodeSessionSummary>;
  attachSession(sessionId: string): Promise<NodeAttachedSession>;
  topologySnapshot(sessionId: string): Promise<NodeTopologySnapshot>;
  screenSnapshot(sessionId: string, paneId: string): Promise<NodeScreenSnapshot>;
}

export interface NativeBindingModule {
  TerminalNodeClient: {
    fromRuntimeSlug(slug: string): NativeTerminalNodeClientHandle;
    fromNamespacedAddress(value: string): NativeTerminalNodeClientHandle;
    fromFilesystemPath(value: string): NativeTerminalNodeClientHandle;
  };
}

export declare function resolveNativeBindingPath(
  options?: NativeBindingLoadOptions,
): string;

export declare function loadNativeBinding(
  options?: NativeBindingLoadOptions,
): NativeBindingModule;

export declare class TerminalNodeClient
  implements NativeTerminalNodeClientHandle
{
  private constructor(inner: NativeTerminalNodeClientHandle);

  static fromRuntimeSlug(
    slug: string,
    options?: NativeBindingLoadOptions,
  ): TerminalNodeClient;

  static fromNamespacedAddress(
    value: string,
    options?: NativeBindingLoadOptions,
  ): TerminalNodeClient;

  static fromFilesystemPath(
    value: string,
    options?: NativeBindingLoadOptions,
  ): TerminalNodeClient;

  get address(): string;
  bindingVersion(): NodeBindingVersion;
  handshakeInfo(): Promise<NodeHandshakeInfo>;
  listSessions(): Promise<NodeSessionSummary[]>;
  createNativeSession(
    request?: NodeCreateSessionRequest,
  ): Promise<NodeSessionSummary>;
  attachSession(sessionId: string): Promise<NodeAttachedSession>;
  topologySnapshot(sessionId: string): Promise<NodeTopologySnapshot>;
  screenSnapshot(sessionId: string, paneId: string): Promise<NodeScreenSnapshot>;
}

declare const _default: {
  loadNativeBinding: typeof loadNativeBinding;
  resolveNativeBindingPath: typeof resolveNativeBindingPath;
  TerminalNodeClient: typeof TerminalNodeClient;
};

export default _default;
