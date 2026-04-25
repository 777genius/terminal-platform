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
export type { NodeSessionHealthPhase } from "./bindings/NodeSessionHealthPhase";
export type { NodeSessionHealthReason } from "./bindings/NodeSessionHealthReason";
export type { NodeSessionHealthSnapshot } from "./bindings/NodeSessionHealthSnapshot";
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
import type { NodeSessionHealthSnapshot } from "./bindings/NodeSessionHealthSnapshot";
import type { NodeSessionRoute } from "./bindings/NodeSessionRoute";
import type { NodeSessionSummary } from "./bindings/NodeSessionSummary";
import type { NodeSubscriptionEvent } from "./bindings/NodeSubscriptionEvent";
import type { NodeSubscriptionSpec } from "./bindings/NodeSubscriptionSpec";
import type { NodeTopologySnapshot } from "./bindings/NodeTopologySnapshot";

export interface NativeBindingLoadOptions {
  addonPath?: string | undefined;
}

export interface TerminalNodeDiagnosticsClient {
  readonly address?: string | undefined;
  bindingVersion(): Promise<NodeBindingVersion> | NodeBindingVersion;
  handshakeInfo(): Promise<NodeHandshakeInfo> | NodeHandshakeInfo;
  backendCapabilities(
    backend: NodeBackendKind,
  ): Promise<NodeBackendCapabilitiesInfo> | NodeBackendCapabilitiesInfo;
}

export interface TerminalNodeEnvironmentReportOptions {
  backends?: NodeBackendKind[] | undefined;
  includeBindingPath?: boolean | undefined;
}

export interface TerminalNodeEnvironmentReportBackend {
  backend: NodeBackendKind;
  promisedInV1: boolean;
  capabilities: NodeBackendCapabilitiesInfo["capabilities"];
}

export interface TerminalNodeEnvironmentReport {
  runtime: {
    platform: string;
    arch: string;
    nodeVersion: string;
  };
  binding: {
    path: string | null;
    version: NodeBindingVersion;
  };
  daemon: {
    address: string | null;
    handshake: NodeHandshakeInfo;
  };
  supportMatrix: {
    currentPlatform: string;
    inV1SupportMatrix: boolean;
    promisedBackends: NodeBackendKind[];
    unpromisedBackends: NodeBackendKind[];
  };
  backends: TerminalNodeEnvironmentReportBackend[];
}

export interface TerminalNodeSubscriptionPumpOptions {
  signal?: AbortSignal | null | undefined;
  onEvent(event: NodeSubscriptionEvent): void | Promise<void>;
}

export type TerminalNodeSessionWatchEvent =
  | { kind: "attached"; attached: NodeAttachedSession }
  | { kind: "topology_snapshot"; topology: NodeTopologySnapshot }
  | { kind: "session_health_snapshot"; health: NodeSessionHealthSnapshot }
  | { kind: "focused_screen"; screen: NodeScreenSnapshot }
  | { kind: "screen_delta"; delta: NodeScreenDelta };

export interface TerminalNodeSessionWatchOptions {
  signal?: AbortSignal | null | undefined;
  onEvent(event: TerminalNodeSessionWatchEvent): void | Promise<void>;
}

export interface TerminalNodeSessionState {
  session: NodeSessionSummary;
  health: NodeSessionHealthSnapshot;
  topology: NodeTopologySnapshot;
  focusedScreen: NodeScreenSnapshot | null;
}

export interface TerminalNodeSessionStateWatchOptions {
  signal?: AbortSignal | null | undefined;
  onState(state: TerminalNodeSessionState): void | Promise<void>;
}

export type ElectronTerminalNodeInvokeMethod =
  | "attachSession"
  | "backendCapabilities"
  | "bindingVersion"
  | "createNativeSession"
  | "deleteSavedSession"
  | "discoverSessions"
  | "dispatchMuxCommand"
  | "handshakeInfo"
  | "importSession"
  | "listSavedSessions"
  | "listSessions"
  | "pruneSavedSessions"
  | "restoreSavedSession"
  | "savedSession"
  | "screenDelta"
  | "screenSnapshot"
  | "sessionHealthSnapshot"
  | "topologySnapshot";

export interface ElectronBridgeChannels {
  invoke: string;
  sessionStateEvent: string;
  sessionStateStart: string;
  sessionStateStop: string;
}

export interface ElectronWebContentsLike {
  send(
    channel: string,
    payload: ElectronTerminalNodeSessionStateEnvelope,
  ): void;
  isDestroyed?(): boolean;
}

export interface ElectronIpcMainInvokeEventLike {
  sender: ElectronWebContentsLike;
}

export interface ElectronIpcMainLike {
  handle(
    channel: string,
    listener: (
      event: ElectronIpcMainInvokeEventLike,
      payload?: unknown,
    ) => unknown | Promise<unknown>,
  ): void;
  removeHandler(channel: string): void;
}

export interface ElectronIpcRendererEventLike {
  sender?: unknown;
}

export interface ElectronIpcRendererLike {
  invoke(channel: string, payload?: unknown): Promise<unknown>;
  on(
    channel: string,
    listener: (
      event: ElectronIpcRendererEventLike,
      payload: ElectronTerminalNodeSessionStateEnvelope,
    ) => void,
  ): unknown;
  off(
    channel: string,
    listener: (
      event: ElectronIpcRendererEventLike,
      payload: ElectronTerminalNodeSessionStateEnvelope,
    ) => void,
  ): unknown;
}

export interface ElectronMainBridgeOptions {
  ipcMain: ElectronIpcMainLike;
  client: TerminalNodeClient;
  channelPrefix?: string | undefined;
}

export interface ElectronTerminalNodeClientOptions {
  ipcRenderer: ElectronIpcRendererLike;
  channelPrefix?: string | undefined;
}

export interface ElectronContextBridgeLike {
  exposeInMainWorld(
    key: string,
    api: ElectronTerminalNodePreloadApi,
  ): void;
}

export interface ElectronPreloadApiOptions {
  ipcRenderer: ElectronIpcRendererLike;
  channelPrefix?: string | undefined;
}

export interface ElectronPreloadBridgeOptions
  extends ElectronPreloadApiOptions {
  contextBridge: ElectronContextBridgeLike;
  exposeKey?: string | undefined;
}

export interface ElectronTerminalNodeMainBridge {
  readonly channels: ElectronBridgeChannels;
  dispose(): void;
}

export interface ElectronTerminalNodePreloadApi {
  bindingVersion(): Promise<NodeBindingVersion>;
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
  sessionHealthSnapshot(sessionId: string): Promise<NodeSessionHealthSnapshot>;
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
  subscribeSessionState(
    sessionId: string,
    onState: (state: TerminalNodeSessionState) => void | Promise<void>,
    onError?: (error: Error) => void | Promise<void>,
  ): Promise<string>;
  unsubscribeSessionState(subscriptionId: string): Promise<boolean>;
  dispose(): Promise<void>;
}

export type ElectronTerminalNodeSessionStateEnvelope =
  | {
      subscriptionId: string;
      kind: "state";
      state: TerminalNodeSessionState;
    }
  | {
      subscriptionId: string;
      kind: "error";
      error: {
        message: string;
        code?: string | undefined;
      };
    }
  | {
      subscriptionId: string;
      kind: "closed";
    };

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
  sessionHealthSnapshot(sessionId: string): Promise<NodeSessionHealthSnapshot>;
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
export declare function collectEnvironmentReport(
  client: TerminalNodeDiagnosticsClient,
  options?: TerminalNodeEnvironmentReportOptions,
): Promise<TerminalNodeEnvironmentReport>;

export declare function createSessionState(
  attached: NodeAttachedSession,
): TerminalNodeSessionState;

export declare function applyScreenDelta(
  snapshot: NodeScreenSnapshot | null,
  delta: NodeScreenDelta,
): NodeScreenSnapshot;

export declare function reduceSessionWatchEvent(
  state: TerminalNodeSessionState | null,
  event: TerminalNodeSessionWatchEvent,
): TerminalNodeSessionState;

export declare function createElectronMainBridge(
  options: ElectronMainBridgeOptions,
): ElectronTerminalNodeMainBridge;

export declare function createElectronPreloadApi(
  options: ElectronPreloadApiOptions,
): ElectronTerminalNodePreloadApi;

export declare function installElectronPreloadBridge(
  options: ElectronPreloadBridgeOptions,
): ElectronTerminalNodePreloadApi;

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
  sessionHealthSnapshot(sessionId: string): Promise<NodeSessionHealthSnapshot>;
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
  watchTopology(
    sessionId: string,
    options: TerminalNodeSubscriptionPumpOptions,
  ): Promise<void>;
  watchPane(
    sessionId: string,
    paneId: string,
    options: TerminalNodeSubscriptionPumpOptions,
  ): Promise<void>;
  watchSession(
    sessionId: string,
    options: TerminalNodeSessionWatchOptions,
  ): Promise<void>;
  watchSessionState(
    sessionId: string,
    options: TerminalNodeSessionStateWatchOptions,
  ): Promise<void>;
}

export declare class TerminalNodeSubscription
  implements NativeTerminalNodeSubscriptionHandle, AsyncIterable<NodeSubscriptionEvent>
{
  private constructor(inner: NativeTerminalNodeSubscriptionHandle);

  get subscriptionId(): string;
  nextEvent(): Promise<NodeSubscriptionEvent | null>;
  close(): Promise<void>;
  pump(options: TerminalNodeSubscriptionPumpOptions): Promise<void>;
  [Symbol.asyncIterator](): AsyncIterableIterator<NodeSubscriptionEvent>;
}

export declare class ElectronTerminalNodeClient {
  constructor(options: ElectronTerminalNodeClientOptions);

  bindingVersion(): Promise<NodeBindingVersion>;
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
  sessionHealthSnapshot(sessionId: string): Promise<NodeSessionHealthSnapshot>;
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
  watchSessionState(
    sessionId: string,
    options: TerminalNodeSessionStateWatchOptions,
  ): Promise<void>;
}

declare const _default: {
  applyScreenDelta: typeof applyScreenDelta;
  collectEnvironmentReport: typeof collectEnvironmentReport;
  createElectronMainBridge: typeof createElectronMainBridge;
  createElectronPreloadApi: typeof createElectronPreloadApi;
  createSessionState: typeof createSessionState;
  ElectronTerminalNodeClient: typeof ElectronTerminalNodeClient;
  installElectronPreloadBridge: typeof installElectronPreloadBridge;
  loadNativeBinding: typeof loadNativeBinding;
  reduceSessionWatchEvent: typeof reduceSessionWatchEvent;
  resolveNativeBindingPath: typeof resolveNativeBindingPath;
  TerminalNodeClient: typeof TerminalNodeClient;
  TerminalNodeSubscription: typeof TerminalNodeSubscription;
};

export default _default;
