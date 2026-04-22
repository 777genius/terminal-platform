import type * as TerminalPlatformSdk from "../../../../../../.generated/terminal-platform-node/index.mjs";
import type {
  TerminalBackendCapabilitiesInfo,
  TerminalCreateNativeSessionInput,
  TerminalDeleteSavedSessionResponse,
  TerminalHandshakeInfo,
  TerminalMuxCommand,
  TerminalMuxCommandResult,
  TerminalPaneTreeNode,
  TerminalSavedSessionSummary,
  TerminalScreenSnapshot,
  TerminalSessionOrigin,
  TerminalSessionState,
  TerminalSessionSummary,
  TerminalTopologySnapshot,
} from "@features/terminal-workspace-kernel/contracts";
import type {
  TerminalRuntimeDiscoveredSession,
  TerminalRuntimeSessionRoute,
} from "../../../core/application/TerminalRuntimeModels.js";
import {
  buildBackendDegradedSemantics,
  buildDiscoveredSessionDegradedSemantics,
  buildHandshakeDegradedSemantics,
  buildSavedSessionDegradedSemantics,
  buildSessionDegradedSemantics,
} from "@features/terminal-workspace-kernel/contracts";

export function toTerminalHandshakeInfo(
  info: TerminalPlatformSdk.NodeHandshakeInfo,
): TerminalHandshakeInfo {
  const handshakeInfo = {
    handshake: {
      protocol_version: {
        major: info.handshake.protocol_version.major,
        minor: info.handshake.protocol_version.minor,
      },
      binary_version: info.handshake.binary_version,
      daemon_phase: info.handshake.daemon_phase,
      capabilities: {
        ...info.handshake.capabilities,
      },
      available_backends: [...info.handshake.available_backends],
      session_scope: info.handshake.session_scope,
    },
    assessment: {
      can_use: info.assessment.can_use,
      protocol: {
        can_connect: info.assessment.protocol.can_connect,
        status: info.assessment.protocol.status,
      },
      status: info.assessment.status,
    },
  };

  return {
    ...handshakeInfo,
    degradedSemantics: buildHandshakeDegradedSemantics(handshakeInfo),
  };
}

export function toTerminalSessionSummary(
  session: TerminalPlatformSdk.NodeSessionSummary,
): TerminalSessionSummary {
  const summary = {
    session_id: session.session_id,
    origin: toTerminalSessionOrigin(session.route),
    title: session.title,
  };

  return {
    ...summary,
    degradedSemantics: buildSessionDegradedSemantics(summary),
  };
}

export function toTerminalSavedSessionSummary(
  session: TerminalPlatformSdk.NodeSavedSessionSummary,
): TerminalSavedSessionSummary {
  const savedSession = {
    session_id: session.session_id,
    origin: toTerminalSessionOrigin(session.route),
    title: session.title,
    saved_at_ms: toSafeNumber(session.saved_at_ms, "saved_at_ms"),
    manifest: {
      format_version: session.manifest.format_version,
      binary_version: session.manifest.binary_version,
      protocol_major: session.manifest.protocol_major,
      protocol_minor: session.manifest.protocol_minor,
    },
    compatibility: {
      can_restore: session.compatibility.can_restore,
      status: session.compatibility.status,
    },
    has_launch: session.has_launch,
    tab_count: session.tab_count,
    pane_count: session.pane_count,
    restore_semantics: {
      restores_topology: session.restore_semantics.restores_topology,
      restores_focus_state: session.restore_semantics.restores_focus_state,
      restores_tab_titles: session.restore_semantics.restores_tab_titles,
      uses_saved_launch_spec: session.restore_semantics.uses_saved_launch_spec,
      replays_saved_screen_buffers: session.restore_semantics.replays_saved_screen_buffers,
      preserves_process_state: session.restore_semantics.preserves_process_state,
    },
  };

  return {
    ...savedSession,
    degradedSemantics: buildSavedSessionDegradedSemantics(savedSession),
  };
}

export function toTerminalRuntimeDiscoveredSession(
  session: TerminalPlatformSdk.NodeDiscoveredSession,
): TerminalRuntimeDiscoveredSession {
  return {
    route: toRuntimeSessionRoute(session.route),
    title: session.title,
  };
}

export function toTerminalBackendCapabilitiesInfo(
  info: TerminalPlatformSdk.NodeBackendCapabilitiesInfo,
): TerminalBackendCapabilitiesInfo {
  const capabilityInfo = {
    backend: info.backend,
    capabilities: {
      ...info.capabilities,
    },
  };

  return {
    ...capabilityInfo,
    degradedSemantics: buildBackendDegradedSemantics(capabilityInfo),
  };
}

export function toTerminalMuxCommandResult(
  result: TerminalPlatformSdk.NodeMuxCommandResult,
): TerminalMuxCommandResult {
  return {
    changed: result.changed,
  };
}

export function toTerminalSessionState(
  state: TerminalPlatformSdk.TerminalNodeSessionState,
): TerminalSessionState {
  return {
    session: toTerminalSessionSummary(state.session),
    topology: toTerminalTopologySnapshot(state.topology),
    focusedScreen: state.focusedScreen ? toTerminalScreenSnapshot(state.focusedScreen) : null,
  };
}

export function toTerminalDeleteSavedSessionResponse(
  result: TerminalPlatformSdk.NodeDeleteSavedSessionResult,
): TerminalDeleteSavedSessionResponse {
  return {
    sessionId: result.session_id,
  };
}

export function toSdkCreateNativeSessionRequest(
  input: TerminalCreateNativeSessionInput,
): TerminalPlatformSdk.NodeCreateSessionRequest {
  return {
    title: input.title ?? null,
    launch: input.launch
      ? {
          program: input.launch.program,
          args: [...input.launch.args],
          cwd: input.launch.cwd ?? null,
        }
      : null,
  };
}

export function toSdkImportSessionInput(input: {
  route: TerminalRuntimeSessionRoute;
  title?: string;
}): {
  route: TerminalPlatformSdk.NodeSessionRoute;
  title?: string;
} {
  return {
    route: toSdkSessionRoute(input.route),
    ...(input.title ? { title: input.title } : {}),
  };
}

export function toSdkMuxCommand(
  command: TerminalMuxCommand,
): TerminalPlatformSdk.NodeMuxCommand {
  switch (command.kind) {
    case "split_pane":
      return {
        kind: command.kind,
        pane_id: command.pane_id,
        direction: command.direction,
      };
    case "close_pane":
      return {
        kind: command.kind,
        pane_id: command.pane_id,
      };
    case "focus_pane":
      return {
        kind: command.kind,
        pane_id: command.pane_id,
      };
    case "resize_pane":
      return {
        kind: command.kind,
        pane_id: command.pane_id,
        rows: command.rows,
        cols: command.cols,
      };
    case "new_tab":
      return {
        kind: command.kind,
        title: command.title,
      };
    case "close_tab":
      return {
        kind: command.kind,
        tab_id: command.tab_id,
      };
    case "focus_tab":
      return {
        kind: command.kind,
        tab_id: command.tab_id,
      };
    case "rename_tab":
      return {
        kind: command.kind,
        tab_id: command.tab_id,
        title: command.title,
      };
    case "send_input":
      return {
        kind: command.kind,
        pane_id: command.pane_id,
        data: command.data,
      };
    case "send_paste":
      return {
        kind: command.kind,
        pane_id: command.pane_id,
        data: command.data,
      };
    case "detach":
      return {
        kind: command.kind,
      };
    case "save_session":
      return {
        kind: command.kind,
      };
    case "override_layout":
      return {
        kind: command.kind,
        tab_id: command.tab_id,
        root: toSdkPaneTreeNode(command.root),
      };
    default:
      return assertNever(command);
  }
}

function toTerminalSessionOrigin(
  route: TerminalPlatformSdk.NodeSessionRoute,
): TerminalSessionOrigin {
  return {
    backend: route.backend,
    authority: route.authority,
    foreignReferenceLabel: route.external?.namespace ?? null,
  };
}

function toRuntimeSessionRoute(
  route: TerminalPlatformSdk.NodeSessionRoute,
): TerminalRuntimeSessionRoute {
  return {
    backend: route.backend,
    authority: route.authority,
    external: route.external
      ? {
          namespace: route.external.namespace,
          value: route.external.value,
        }
      : null,
  };
}

function toSdkSessionRoute(
  route: TerminalRuntimeSessionRoute,
): TerminalPlatformSdk.NodeSessionRoute {
  return {
    backend: route.backend,
    authority: route.authority,
    external: route.external
      ? {
          namespace: route.external.namespace,
          value: route.external.value,
        }
      : null,
  };
}

function toTerminalTopologySnapshot(
  topology: TerminalPlatformSdk.NodeTopologySnapshot,
): TerminalTopologySnapshot {
  return {
    session_id: topology.session_id,
    backend_kind: topology.backend_kind,
    tabs: topology.tabs.map((tab: TerminalPlatformSdk.NodeTabSnapshot) => ({
      tab_id: tab.tab_id,
      title: tab.title,
      root: toTerminalPaneTreeNode(tab.root),
      focused_pane: tab.focused_pane,
    })),
    focused_tab: topology.focused_tab,
  };
}

function toTerminalPaneTreeNode(
  node: TerminalPlatformSdk.NodePaneTreeNode,
): TerminalPaneTreeNode {
  if (node.kind === "leaf") {
    return {
      kind: "leaf",
      pane_id: node.pane_id,
    };
  }

  return {
    kind: "split",
    direction: node.direction,
    first: toTerminalPaneTreeNode(node.first),
    second: toTerminalPaneTreeNode(node.second),
  };
}

function toTerminalScreenSnapshot(
  screen: TerminalPlatformSdk.NodeScreenSnapshot,
): TerminalScreenSnapshot {
  return {
    pane_id: screen.pane_id,
    sequence: screen.sequence.toString(),
    rows: screen.rows,
    cols: screen.cols,
    source: screen.source,
    surface: {
      title: screen.surface.title,
      cursor: screen.surface.cursor
        ? {
            row: screen.surface.cursor.row,
            col: screen.surface.cursor.col,
          }
        : null,
      lines: screen.surface.lines.map((line: TerminalPlatformSdk.NodeScreenLine) => ({
        text: line.text,
      })),
    },
  };
}

function toSdkPaneTreeNode(
  node: TerminalPaneTreeNode,
): TerminalPlatformSdk.NodePaneTreeNode {
  if (node.kind === "leaf") {
    return {
      kind: "leaf",
      pane_id: node.pane_id,
    };
  }

  return {
    kind: "split",
    direction: node.direction,
    first: toSdkPaneTreeNode(node.first),
    second: toSdkPaneTreeNode(node.second),
  };
}

function toSafeNumber(value: bigint, fieldName: string): number {
  const numeric = Number(value);
  if (!Number.isSafeInteger(numeric)) {
    throw new Error(`Terminal contract field ${fieldName} exceeds JS safe integer range`);
  }

  return numeric;
}

function assertNever(value: never): never {
  throw new Error(`Unsupported mux command: ${JSON.stringify(value)}`);
}
