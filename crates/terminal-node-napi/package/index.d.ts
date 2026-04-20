export type { NodeAttachedSession } from "./bindings/NodeAttachedSession";
export type { NodeBackendCapabilities } from "./bindings/NodeBackendCapabilities";
export type { NodeBackendCapabilitiesInfo } from "./bindings/NodeBackendCapabilitiesInfo";
export type { NodeBackendKind } from "./bindings/NodeBackendKind";
export type { NodeBindingVersion } from "./bindings/NodeBindingVersion";
export type { NodeCreateSessionRequest } from "./bindings/NodeCreateSessionRequest";
export type { NodeDaemonCapabilities } from "./bindings/NodeDaemonCapabilities";
export type { NodeDaemonPhase } from "./bindings/NodeDaemonPhase";
export type { NodeDeleteSavedSessionResult } from "./bindings/NodeDeleteSavedSessionResult";
export type { NodeDiscoveredSession } from "./bindings/NodeDiscoveredSession";
export type { NodeExternalSessionRef } from "./bindings/NodeExternalSessionRef";
export type { NodeHandshake } from "./bindings/NodeHandshake";
export type { NodeHandshakeAssessment } from "./bindings/NodeHandshakeAssessment";
export type { NodeHandshakeAssessmentStatus } from "./bindings/NodeHandshakeAssessmentStatus";
export type { NodeHandshakeInfo } from "./bindings/NodeHandshakeInfo";
export type { NodeMuxCommand } from "./bindings/NodeMuxCommand";
export type { NodeMuxCommandResult } from "./bindings/NodeMuxCommandResult";
export type { NodePaneSplit } from "./bindings/NodePaneSplit";
export type { NodePaneTreeNode } from "./bindings/NodePaneTreeNode";
export type { NodeProjectionSource } from "./bindings/NodeProjectionSource";
export type { NodeProtocolCompatibility } from "./bindings/NodeProtocolCompatibility";
export type { NodeProtocolCompatibilityStatus } from "./bindings/NodeProtocolCompatibilityStatus";
export type { NodeProtocolVersion } from "./bindings/NodeProtocolVersion";
export type { NodePruneSavedSessionsResult } from "./bindings/NodePruneSavedSessionsResult";
export type { NodeRestoredSession } from "./bindings/NodeRestoredSession";
export type { NodeRouteAuthority } from "./bindings/NodeRouteAuthority";
export type { NodeScreenCursor } from "./bindings/NodeScreenCursor";
export type { NodeScreenDelta } from "./bindings/NodeScreenDelta";
export type { NodeScreenLine } from "./bindings/NodeScreenLine";
export type { NodeScreenLinePatch } from "./bindings/NodeScreenLinePatch";
export type { NodeScreenPatch } from "./bindings/NodeScreenPatch";
export type { NodeScreenSnapshot } from "./bindings/NodeScreenSnapshot";
export type { NodeScreenSurface } from "./bindings/NodeScreenSurface";
export type { NodeSavedSessionCompatibility } from "./bindings/NodeSavedSessionCompatibility";
export type { NodeSavedSessionCompatibilityStatus } from "./bindings/NodeSavedSessionCompatibilityStatus";
export type { NodeSavedSessionManifest } from "./bindings/NodeSavedSessionManifest";
export type { NodeSavedSessionRecord } from "./bindings/NodeSavedSessionRecord";
export type { NodeSavedSessionRestoreSemantics } from "./bindings/NodeSavedSessionRestoreSemantics";
export type { NodeSavedSessionSummary } from "./bindings/NodeSavedSessionSummary";
export type { NodeSessionRoute } from "./bindings/NodeSessionRoute";
export type { NodeSessionSummary } from "./bindings/NodeSessionSummary";
export type { NodeShellLaunchSpec } from "./bindings/NodeShellLaunchSpec";
export type { NodeSplitDirection } from "./bindings/NodeSplitDirection";
export type { NodeSubscriptionEvent } from "./bindings/NodeSubscriptionEvent";
export type { NodeSubscriptionMeta } from "./bindings/NodeSubscriptionMeta";
export type { NodeSubscriptionSpec } from "./bindings/NodeSubscriptionSpec";
export type { NodeTabSnapshot } from "./bindings/NodeTabSnapshot";
export type { NodeTopologySnapshot } from "./bindings/NodeTopologySnapshot";

import type { NodeAttachedSession } from "./bindings/NodeAttachedSession";
import type { NodeBackendCapabilitiesInfo } from "./bindings/NodeBackendCapabilitiesInfo";
import type { NodeBackendKind } from "./bindings/NodeBackendKind";
import type { NodeBindingVersion } from "./bindings/NodeBindingVersion";
import type { NodeCreateSessionRequest } from "./bindings/NodeCreateSessionRequest";
import type { NodeDeleteSavedSessionResult } from "./bindings/NodeDeleteSavedSessionResult";
import type { NodeDiscoveredSession } from "./bindings/NodeDiscoveredSession";
import type { NodeHandshakeInfo } from "./bindings/NodeHandshakeInfo";
import type { NodeMuxCommand } from "./bindings/NodeMuxCommand";
import type { NodeMuxCommandResult } from "./bindings/NodeMuxCommandResult";
import type { NodePruneSavedSessionsResult } from "./bindings/NodePruneSavedSessionsResult";
import type { NodeRestoredSession } from "./bindings/NodeRestoredSession";
import type { NodeScreenDelta } from "./bindings/NodeScreenDelta";
import type { NodeScreenSnapshot } from "./bindings/NodeScreenSnapshot";
import type { NodeSavedSessionRecord } from "./bindings/NodeSavedSessionRecord";
import type { NodeSavedSessionSummary } from "./bindings/NodeSavedSessionSummary";
import type { NodeSessionRoute } from "./bindings/NodeSessionRoute";
import type { NodeSessionSummary } from "./bindings/NodeSessionSummary";
import type { NodeSubscriptionEvent } from "./bindings/NodeSubscriptionEvent";
import type { NodeSubscriptionSpec } from "./bindings/NodeSubscriptionSpec";
import type { NodeTopologySnapshot } from "./bindings/NodeTopologySnapshot";

export interface NativeBindingLoadOptions {
  addonPath?: string | undefined;
}

export interface NativeTargetDescriptor {
  platform: string;
  arch: string;
  libc: string | null;
  file: string;
  packageVersion: string;
}

export interface NativeAddonManifest {
  schemaVersion: 1;
  packageVersion: string;
  targets: NativeTargetDescriptor[];
}

export interface NativeTerminalNodeClientHandle {
  readonly address: string;
  bindingVersion(): NodeBindingVersion;
  handshakeInfo(): Promise<NodeHandshakeInfo>;
  listSessions(): Promise<NodeSessionSummary[]>;
  listSavedSessions(): Promise<NodeSavedSessionSummary[]>;
  discoverSessions(backend: NodeBackendKind): Promise<NodeDiscoveredSession[]>;
  backendCapabilities(
    backend: NodeBackendKind,
  ): Promise<NodeBackendCapabilitiesInfo>;
  createNativeSession(request: NodeCreateSessionRequest): Promise<NodeSessionSummary>;
  importSession(
    route: NodeSessionRoute,
    title?: string | null,
  ): Promise<NodeSessionSummary>;
  savedSession(sessionId: string): Promise<NodeSavedSessionRecord>;
  deleteSavedSession(sessionId: string): Promise<NodeDeleteSavedSessionResult>;
  pruneSavedSessions(
    keepLatest: number,
  ): Promise<NodePruneSavedSessionsResult>;
  restoreSavedSession(sessionId: string): Promise<NodeRestoredSession>;
  attachSession(sessionId: string): Promise<NodeAttachedSession>;
  topologySnapshot(sessionId: string): Promise<NodeTopologySnapshot>;
  screenSnapshot(sessionId: string, paneId: string): Promise<NodeScreenSnapshot>;
  screenDelta(
    sessionId: string,
    paneId: string,
    fromSequence: number,
  ): Promise<NodeScreenDelta>;
  dispatchMuxCommand(
    sessionId: string,
    command: NodeMuxCommand,
  ): Promise<NodeMuxCommandResult>;
  openSubscription(
    sessionId: string,
    spec: NodeSubscriptionSpec,
  ): Promise<NativeTerminalNodeSubscriptionHandle>;
}

export interface NativeTerminalNodeSubscriptionHandle {
  readonly subscriptionId: string;
  nextEvent(): Promise<NodeSubscriptionEvent | null>;
  close(): Promise<void>;
}

export interface NativeBindingModule {
  TerminalNodeClient: {
    fromRuntimeSlug(slug: string): NativeTerminalNodeClientHandle;
    fromNamespacedAddress(value: string): NativeTerminalNodeClientHandle;
    fromFilesystemPath(value: string): NativeTerminalNodeClientHandle;
  };
  TerminalNodeSubscription: {
    prototype: NativeTerminalNodeSubscriptionHandle;
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
  listSavedSessions(): Promise<NodeSavedSessionSummary[]>;
  discoverSessions(backend: NodeBackendKind): Promise<NodeDiscoveredSession[]>;
  backendCapabilities(
    backend: NodeBackendKind,
  ): Promise<NodeBackendCapabilitiesInfo>;
  createNativeSession(
    request?: NodeCreateSessionRequest,
  ): Promise<NodeSessionSummary>;
  importSession(
    route: NodeSessionRoute,
    title?: string | null,
  ): Promise<NodeSessionSummary>;
  savedSession(sessionId: string): Promise<NodeSavedSessionRecord>;
  deleteSavedSession(sessionId: string): Promise<NodeDeleteSavedSessionResult>;
  pruneSavedSessions(
    keepLatest: number,
  ): Promise<NodePruneSavedSessionsResult>;
  restoreSavedSession(sessionId: string): Promise<NodeRestoredSession>;
  attachSession(sessionId: string): Promise<NodeAttachedSession>;
  topologySnapshot(sessionId: string): Promise<NodeTopologySnapshot>;
  screenSnapshot(sessionId: string, paneId: string): Promise<NodeScreenSnapshot>;
  screenDelta(
    sessionId: string,
    paneId: string,
    fromSequence: number,
  ): Promise<NodeScreenDelta>;
  dispatchMuxCommand(
    sessionId: string,
    command: NodeMuxCommand,
  ): Promise<NodeMuxCommandResult>;
  openSubscription(
    sessionId: string,
    spec: NodeSubscriptionSpec,
  ): Promise<TerminalNodeSubscription>;
  subscribeTopology(sessionId: string): Promise<TerminalNodeSubscription>;
  subscribePane(
    sessionId: string,
    paneId: string,
  ): Promise<TerminalNodeSubscription>;
}

export declare class TerminalNodeSubscription
  implements NativeTerminalNodeSubscriptionHandle, AsyncIterable<NodeSubscriptionEvent>
{
  private constructor(inner: NativeTerminalNodeSubscriptionHandle);

  get subscriptionId(): string;
  nextEvent(): Promise<NodeSubscriptionEvent | null>;
  close(): Promise<void>;
  [Symbol.asyncIterator](): AsyncIterableIterator<NodeSubscriptionEvent>;
}

declare const _default: {
  loadNativeBinding: typeof loadNativeBinding;
  resolveNativeBindingPath: typeof resolveNativeBindingPath;
  TerminalNodeClient: typeof TerminalNodeClient;
  TerminalNodeSubscription: typeof TerminalNodeSubscription;
};

export default _default;
