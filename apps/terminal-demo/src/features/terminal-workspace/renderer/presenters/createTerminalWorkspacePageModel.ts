import type {
  TerminalBackendCapabilitiesInfo,
  TerminalBackendKind,
  TerminalDegradedReason,
  TerminalDiscoveredSession,
  TerminalHandshakeInfo,
  TerminalSavedSessionSummary,
  TerminalSessionState,
  TerminalSessionSummary,
} from "../../contracts/terminal-workspace-contracts.js";
import type {
  TerminalWorkspaceSessionStreamHealth,
} from "../../core/application/index.js";
import { compactId } from "../utils/compactId.js";
import type {
  TerminalWorkspaceBadgeModel,
  TerminalWorkspaceBannerModel,
  TerminalWorkspaceCapabilitiesModel,
  TerminalWorkspaceDegradedReasonModel,
  TerminalWorkspaceDiscoveredGroupModel,
  TerminalWorkspacePageModel,
  TerminalWorkspacePaneTreeNodeModel,
  TerminalWorkspaceSavedSessionItemModel,
  TerminalWorkspaceScreenModel,
  TerminalWorkspaceSessionItemModel,
  TerminalWorkspaceTopologyTabModel,
  TerminalWorkspaceTone,
} from "../view-models/TerminalWorkspacePageModel.js";

interface TerminalWorkspacePageModelInput {
  controlPlaneUrl: string;
  sessionStreamUrl: string;
  runtimeSlug: string;
  status: "idle" | "loading" | "ready" | "error";
  sessionStatus: "idle" | "connecting" | "ready" | "error";
  sessionStreamHealth: TerminalWorkspaceSessionStreamHealth;
  error: string | null;
  actionError: string | null;
  actionDegradedReason: TerminalDegradedReason | null;
  handshake: TerminalHandshakeInfo | null;
  sessions: TerminalSessionSummary[];
  discoveredSessions: {
    tmux?: TerminalDiscoveredSession[];
    zellij?: TerminalDiscoveredSession[];
  };
  capabilities: Partial<Record<TerminalBackendKind, TerminalBackendCapabilitiesInfo>>;
  activeSessionId: string | null;
  activeSessionState: TerminalSessionState | null;
  createTitleDraft: string;
  createProgramDraft: string;
  createArgsDraft: string;
  createCwdDraft: string;
  inputDraft: string;
  visibleSavedSessions: TerminalSavedSessionSummary[];
  hiddenSavedSessionsCount: number;
  showAllSavedSessions: boolean;
}

export function createTerminalWorkspacePageModel(
  input: TerminalWorkspacePageModelInput,
): TerminalWorkspacePageModel {
  const activeSession = input.sessions.find((session) => session.session_id === input.activeSessionId) ?? null;
  const activeCapabilities = activeSession
    ? input.capabilities[activeSession.origin.backend] ?? null
    : null;
  const activeSessionState = input.activeSessionState;

  return {
    controlPlaneUrl: input.controlPlaneUrl,
    sessionStreamUrl: input.sessionStreamUrl,
    runtimeSlug: input.runtimeSlug,
    statusBadge: {
      label: `status ${input.status}`,
      tone: input.status === "error" ? "danger" : "brand",
    },
    sessionStatusBadge: {
      label: `session ${input.sessionStatus}`,
      tone: input.sessionStatus === "error" ? "danger" : "neutral",
    },
    sessionStreamBadge: {
      label: `stream ${input.sessionStreamHealth.phase}`,
      tone: streamBadgeTone(input.sessionStreamHealth.phase),
    },
    gatewayInfo: [
      { label: "Runtime", value: input.runtimeSlug },
      { label: "Control", value: input.controlPlaneUrl },
      { label: "Stream", value: input.sessionStreamUrl },
      ...(input.handshake
        ? [
            { label: "Binary", value: input.handshake.handshake.binary_version },
            { label: "Phase", value: input.handshake.handshake.daemon_phase },
          ]
        : []),
    ],
    handshakeDegradedReasons: toDegradedReasonModels(input.handshake?.degradedSemantics ?? []),
    createForm: {
      title: input.createTitleDraft,
      program: input.createProgramDraft,
      args: input.createArgsDraft,
      cwd: input.createCwdDraft,
    },
    sessionItems: input.sessions.map((session) => toSessionItemModel(session, input.activeSessionId)),
    discoveredGroups: toDiscoveredGroupModels(input.discoveredSessions),
    savedSessionItems: input.visibleSavedSessions.map(toSavedSessionItemModel),
    hiddenSavedSessionsCount: input.hiddenSavedSessionsCount,
    showAllSavedSessions: input.showAllSavedSessions,
    activeSessionTitle: activeSession?.title ?? "Pick a session to inspect",
    activeBackendBadge: activeSession
      ? {
          label: activeSession.origin.backend,
          tone: "brand",
        }
      : null,
    errorBanner: input.error
      ? {
          tone: "default",
          title: null,
          detail: input.error,
        }
      : null,
    sessionStreamBanner: toSessionStreamBanner(input.sessionStreamHealth),
    actionDegradedBanner: input.actionDegradedReason
      ? {
          tone: "warning",
          title: input.actionDegradedReason.summary,
          detail: input.actionDegradedReason.detail,
        }
      : null,
    actionErrorBanner: input.actionError
      ? {
          tone: "subtle",
          title: null,
          detail: input.actionError,
        }
      : null,
    toolbar: {
      canNewTab: Boolean(input.activeSessionId && activeCapabilities?.capabilities.tab_create),
      canSplit: Boolean(input.activeSessionId && activeCapabilities?.capabilities.pane_split),
      canSave: Boolean(input.activeSessionId && activeCapabilities?.capabilities.explicit_session_save),
    },
    topologyTabs: activeSessionState
      ? activeSessionState.topology.tabs.map((tab) => toTopologyTabModel(tab, activeSessionState))
      : [],
    screen: toScreenModel(activeSessionState),
    input: {
      draft: input.inputDraft,
      canWrite: Boolean(input.activeSessionId && activeCapabilities?.capabilities.pane_input_write),
    },
    capabilities: toCapabilitiesModel(activeSession, activeCapabilities),
  };
}

function toSessionItemModel(
  session: TerminalSessionSummary,
  activeSessionId: string | null,
): TerminalWorkspaceSessionItemModel {
  return {
    sessionId: session.session_id,
    title: session.title ?? "Untitled session",
    meta: `${session.origin.backend} - ${compactId(session.session_id)}`,
    isActive: session.session_id === activeSessionId,
  };
}

function toDiscoveredGroupModels(input: {
  tmux?: TerminalDiscoveredSession[];
  zellij?: TerminalDiscoveredSession[];
}): TerminalWorkspaceDiscoveredGroupModel[] {
  return [
    {
      key: "tmux",
      label: "tmux",
      sessions: (input.tmux ?? []).map((session) => ({
        importHandle: session.importHandle,
        backend: session.backend as "tmux",
        title: session.title ?? "Untitled foreign session",
        sourceLabel: session.sourceLabel,
        degradedReasons: toDegradedReasonModels(session.degradedSemantics),
      })),
      emptyText: "No tmux sessions discovered.",
    },
    {
      key: "zellij",
      label: "zellij",
      sessions: (input.zellij ?? []).map((session) => ({
        importHandle: session.importHandle,
        backend: session.backend as "zellij",
        title: session.title ?? "Untitled foreign session",
        sourceLabel: session.sourceLabel,
        degradedReasons: toDegradedReasonModels(session.degradedSemantics),
      })),
      emptyText: "No zellij sessions discovered.",
    },
  ];
}

function toSavedSessionItemModel(session: TerminalSavedSessionSummary): TerminalWorkspaceSavedSessionItemModel {
  return {
    sessionId: session.session_id,
    title: session.title ?? "Untitled save",
    meta: `${session.origin.backend} - ${new Date(session.saved_at_ms).toLocaleString()}`,
    degradedReasons: toDegradedReasonModels(session.degradedSemantics),
    canRestore: session.compatibility.can_restore,
  };
}

function toTopologyTabModel(
  tab: TerminalSessionState["topology"]["tabs"][number],
  state: TerminalSessionState,
): TerminalWorkspaceTopologyTabModel {
  return {
    tabId: tab.tab_id,
    title: tab.title ?? "Untitled tab",
    isFocused: tab.tab_id === state.topology.focused_tab,
    root: toPaneTreeNodeModel(tab.root, tab.focused_pane),
  };
}

function toPaneTreeNodeModel(
  node: TerminalSessionState["topology"]["tabs"][number]["root"],
  focusedPaneId: string | null,
): TerminalWorkspacePaneTreeNodeModel {
  if (node.kind === "leaf") {
    return {
      kind: "leaf",
      paneId: node.pane_id,
      label: `pane ${compactId(node.pane_id)}`,
      isFocused: node.pane_id === focusedPaneId,
    };
  }

  return {
    kind: "split",
    direction: node.direction,
    first: toPaneTreeNodeModel(node.first, focusedPaneId),
    second: toPaneTreeNodeModel(node.second, focusedPaneId),
  };
}

function toScreenModel(state: TerminalSessionState | null): TerminalWorkspaceScreenModel | null {
  if (!state?.focusedScreen) {
    return null;
  }

  return {
    sizeBadge: {
      label: `${state.focusedScreen.rows}x${state.focusedScreen.cols}`,
      tone: "neutral",
    },
    sequenceBadge: {
      label: `seq ${state.focusedScreen.sequence}`,
      tone: "neutral",
    },
    meta: [
      `pane ${compactId(state.focusedScreen.pane_id)}`,
      state.focusedScreen.source,
      `cursor ${state.focusedScreen.surface.cursor?.row ?? "-"}:${state.focusedScreen.surface.cursor?.col ?? "-"}`,
    ],
    lines: state.focusedScreen.surface.lines.map((line, index) => ({
      key: `${index}-${line.text}`,
      gutter: String(index + 1).padStart(2, "0"),
      text: line.text || " ",
    })),
  };
}

function toCapabilitiesModel(
  activeSession: TerminalSessionSummary | null,
  info: TerminalBackendCapabilitiesInfo | null,
): TerminalWorkspaceCapabilitiesModel | null {
  if (!info) {
    return null;
  }

  const badges = Object.entries(info.capabilities)
    .filter(([, supported]) => supported)
    .map(([name]) => ({
      label: name.replaceAll("_", " "),
      tone: "neutral" as const,
    }));
  const degradedReasons = toDegradedReasonModels([
    ...(activeSession?.degradedSemantics ?? []),
    ...info.degradedSemantics,
  ]);

  return {
    badges,
    degradedReasons,
  };
}

function toSessionStreamBanner(
  health: TerminalWorkspaceSessionStreamHealth,
): TerminalWorkspaceBannerModel | null {
  switch (health.phase) {
    case "connecting":
      return {
        tone: "subtle",
        title: null,
        detail: "Attaching live session stream.",
      };
    case "reconnecting":
      return {
        tone: "warning",
        title: "Live session stream reconnecting",
        detail: `Showing the last projected snapshot while the data plane reconnects.${health.reconnectAttempts > 0 ? ` Retry ${health.reconnectAttempts}.` : ""}${health.lastError ? ` ${health.lastError}` : ""}`,
      };
    case "error":
      return {
        tone: "default",
        title: "Live session stream failed",
        detail: health.lastError ?? "Unknown stream error",
      };
    case "idle":
    case "ready":
      return null;
    default:
      return null;
  }
}

function toDegradedReasonModels(reasons: TerminalDegradedReason[]): TerminalWorkspaceDegradedReasonModel[] {
  return reasons.map((reason) => ({
    id: `${reason.scope}-${reason.code}`,
    badge: {
      label: reason.summary,
      tone: degradedTone(reason.severity),
    },
    detail: reason.detail,
  }));
}

function degradedTone(severity: TerminalDegradedReason["severity"]): TerminalWorkspaceTone {
  switch (severity) {
    case "error":
      return "danger";
    case "warning":
      return "brand";
    case "info":
      return "neutral";
    default:
      return "neutral";
  }
}

function streamBadgeTone(
  phase: TerminalWorkspaceSessionStreamHealth["phase"],
): TerminalWorkspaceTone {
  switch (phase) {
    case "ready":
      return "neutral";
    case "connecting":
    case "reconnecting":
      return "brand";
    case "error":
      return "danger";
    case "idle":
      return "neutral";
    default:
      return "neutral";
  }
}
