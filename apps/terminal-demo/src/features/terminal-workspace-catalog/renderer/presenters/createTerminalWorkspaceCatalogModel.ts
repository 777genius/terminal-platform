import type {
  TerminalBackendCapabilitiesInfo,
  TerminalBackendKind,
  TerminalDegradedReason,
  TerminalDiscoveredSession,
  TerminalHandshakeInfo,
  TerminalRuntimeWorkspaceFacade,
  TerminalSessionSummary,
} from "@features/terminal-workspace-kernel/contracts";
import type { TerminalWorkspaceCatalogFormState } from "../../core/application/index.js";
import { compactId } from "../utils/compactId.js";
import type {
  TerminalWorkspaceCatalogBadgeModel,
  TerminalWorkspaceCatalogDegradedReasonModel,
  TerminalWorkspaceCatalogDiscoveredGroupModel,
  TerminalWorkspaceCatalogModel,
  TerminalWorkspaceCatalogSessionItemModel,
} from "../view-models/TerminalWorkspaceCatalogModel.js";

export function createTerminalWorkspaceCatalogModel(input: {
  runtime: TerminalRuntimeWorkspaceFacade;
  form: TerminalWorkspaceCatalogFormState;
}): TerminalWorkspaceCatalogModel {
  const state = input.runtime.state;

  return {
    statusBadge: {
      label: `status ${state.status}`,
      tone: state.status === "error" ? "danger" : "brand",
    },
    sessionStatusBadge: {
      label: `session ${state.sessionStatus}`,
      tone: state.sessionStatus === "error" ? "danger" : "neutral",
    },
    sessionStreamBadge: {
      label: `stream ${state.sessionStreamHealth.phase}`,
      tone: streamBadgeTone(state.sessionStreamHealth.phase),
    },
    gatewayInfo: [
      { label: "Runtime", value: input.runtime.transport.runtimeSlug },
      { label: "Control", value: input.runtime.transport.controlPlaneUrl },
      { label: "Stream", value: input.runtime.transport.sessionStreamUrl },
      ...(state.handshake
        ? [
            { label: "Binary", value: state.handshake.handshake.binary_version },
            { label: "Phase", value: state.handshake.handshake.daemon_phase },
          ]
        : []),
    ],
    handshakeDegradedReasons: toDegradedReasonModels(state.handshake?.degradedSemantics ?? []),
    createForm: {
      ...input.form,
    },
    sessionItems: state.sessions.map((session) => toSessionItemModel(session, state.activeSessionId)),
    discoveredGroups: toDiscoveredGroupModels(state.discoveredSessions),
  };
}

function toSessionItemModel(
  session: TerminalSessionSummary,
  activeSessionId: string | null,
): TerminalWorkspaceCatalogSessionItemModel {
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
}): TerminalWorkspaceCatalogDiscoveredGroupModel[] {
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

function toDegradedReasonModels(
  reasons: TerminalDegradedReason[],
): TerminalWorkspaceCatalogDegradedReasonModel[] {
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
): TerminalWorkspaceCatalogBadgeModel["tone"] {
  if (severity === "error") {
    return "danger";
  }

  if (severity === "warning") {
    return "neutral";
  }

  return "brand";
}

function streamBadgeTone(
  phase: TerminalRuntimeWorkspaceFacade["state"]["sessionStreamHealth"]["phase"],
): TerminalWorkspaceCatalogBadgeModel["tone"] {
  if (phase === "error") {
    return "danger";
  }

  if (phase === "reconnecting") {
    return "neutral";
  }

  return "brand";
}
