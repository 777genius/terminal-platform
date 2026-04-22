import type {
  TerminalBackendCapabilitiesInfo,
  TerminalBackendKind,
  TerminalDegradedReason,
  TerminalRuntimeWorkspaceFacade,
  TerminalSessionState,
  TerminalSessionSummary,
} from "@features/terminal-workspace-kernel/contracts";
import { compactId } from "../utils/compactId.js";
import type {
  TerminalActiveSessionBadgeModel,
  TerminalActiveSessionCapabilitiesModel,
  TerminalActiveSessionDegradedReasonModel,
  TerminalActiveSessionModel,
  TerminalActiveSessionPaneTreeNodeModel,
  TerminalActiveSessionScreenModel,
  TerminalActiveSessionTopologyTabModel,
} from "../view-models/TerminalActiveSessionModel.js";

export function createTerminalActiveSessionModel(
  runtime: TerminalRuntimeWorkspaceFacade,
): TerminalActiveSessionModel {
  const state = runtime.state;
  const activeSession = state.sessions.find((session) => session.session_id === state.activeSessionId) ?? null;
  const activeCapabilities = activeSession
    ? state.capabilities[activeSession.origin.backend] ?? null
    : null;
  const activeSessionState = state.activeSessionState;

  return {
    activeSessionTitle: activeSession?.title ?? "Pick a session to inspect",
    activeBackendBadge: activeSession
      ? {
          label: activeSession.origin.backend,
          tone: "brand",
        }
      : null,
    errorBanner: state.error
      ? {
          tone: "default",
          title: null,
          detail: state.error,
        }
      : null,
    sessionStreamBanner: toSessionStreamBanner(state.sessionStreamHealth),
    actionDegradedBanner: state.actionDegradedReason
      ? {
          tone: "warning",
          title: state.actionDegradedReason.summary,
          detail: state.actionDegradedReason.detail,
        }
      : null,
    actionErrorBanner: state.actionError
      ? {
          tone: "subtle",
          title: null,
          detail: state.actionError,
        }
      : null,
    toolbar: {
      canNewTab: Boolean(state.activeSessionId && activeCapabilities?.capabilities.tab_create),
      canSplit: Boolean(state.activeSessionId && activeCapabilities?.capabilities.pane_split),
      canSave: Boolean(state.activeSessionId && activeCapabilities?.capabilities.explicit_session_save),
    },
    topologyTabs: activeSessionState
      ? activeSessionState.topology.tabs.map((tab) => toTopologyTabModel(tab, activeSessionState))
      : [],
    screen: toScreenModel(activeSessionState),
    capabilities: toCapabilitiesModel(activeSession, activeCapabilities),
  };
}

function toTopologyTabModel(
  tab: TerminalSessionState["topology"]["tabs"][number],
  state: TerminalSessionState,
): TerminalActiveSessionTopologyTabModel {
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
): TerminalActiveSessionPaneTreeNodeModel {
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

function toScreenModel(state: TerminalSessionState | null): TerminalActiveSessionScreenModel | null {
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
      text: line.text,
    })),
  };
}

function toCapabilitiesModel(
  session: TerminalSessionSummary | null,
  info: TerminalBackendCapabilitiesInfo | null,
): TerminalActiveSessionCapabilitiesModel | null {
  if (!session || !info) {
    return null;
  }

  return {
    badges: [
      { label: `${info.backend} caps`, tone: "brand" },
      { label: info.capabilities.rendered_viewport_snapshot ? "snapshot yes" : "snapshot no", tone: info.capabilities.rendered_viewport_snapshot ? "brand" : "neutral" },
      { label: info.capabilities.raw_output_stream ? "raw yes" : "raw no", tone: info.capabilities.raw_output_stream ? "brand" : "neutral" },
    ],
    degradedReasons: toDegradedReasonModels([
      ...session.degradedSemantics,
      ...info.degradedSemantics,
    ]),
  };
}

function toSessionStreamBanner(
  health: TerminalRuntimeWorkspaceFacade["state"]["sessionStreamHealth"],
) {
  if (health.phase === "ready" || health.phase === "idle") {
    return null;
  }

  if (health.phase === "connecting") {
    return {
      tone: "subtle" as const,
      title: "Live session stream connecting",
      detail: "Session stream is attaching to the selected session.",
    };
  }

  if (health.phase === "reconnecting") {
    return {
      tone: "warning" as const,
      title: "Live session stream reconnecting",
      detail: health.lastError ?? "Session stream is reconnecting after a transient disconnect.",
    };
  }

  return {
    tone: "default" as const,
    title: "Live session stream failed",
    detail: health.lastError ?? "Session stream is unavailable.",
  };
}

function toDegradedReasonModels(
  reasons: TerminalDegradedReason[],
): TerminalActiveSessionDegradedReasonModel[] {
  return reasons.map((reason, index) => ({
    id: `${reason.code}-${index}`,
    badge: {
      label: reason.severity,
      tone: degradedTone(reason.severity),
    },
    detail: reason.detail,
  }));
}

function degradedTone(
  severity: TerminalDegradedReason["severity"],
): TerminalActiveSessionBadgeModel["tone"] {
  if (severity === "error") {
    return "danger";
  }

  if (severity === "warning") {
    return "neutral";
  }

  return "brand";
}
