import type {
  TerminalBackendCapabilitiesInfo,
  TerminalBackendKind,
  TerminalDegradedReason,
  TerminalDiscoveredSession,
  TerminalHandshakeInfo,
  TerminalSavedSessionSummary,
  TerminalSessionSummary,
} from "../../contracts/terminal-workspace-contracts.js";

export type TerminalWorkspaceActionKind =
  | "new_tab"
  | "split_pane"
  | "save_session"
  | "focus_pane"
  | "focus_tab"
  | "send_input";

export function buildHandshakeDegradedSemantics(
  info: Omit<TerminalHandshakeInfo, "degradedSemantics">,
): TerminalDegradedReason[] {
  const reasons: TerminalDegradedReason[] = [];

  if (info.handshake.daemon_phase === "degraded" || info.assessment.status === "degraded") {
    reasons.push({
      code: "daemon_degraded",
      scope: "daemon",
      severity: "warning",
      summary: "Daemon is running in degraded mode",
      detail: "Some terminal semantics are intentionally reduced and should be treated as partial backend truth.",
    });
  }

  if (info.assessment.status === "starting") {
    reasons.push({
      code: "daemon_starting",
      scope: "daemon",
      severity: "info",
      summary: "Daemon is still starting",
      detail: "Capabilities and subscriptions may not be fully ready yet.",
    });
  }

  if (info.assessment.status === "protocol_minor_ahead") {
    reasons.push({
      code: "protocol_minor_ahead",
      scope: "daemon",
      severity: "warning",
      summary: "Daemon protocol is ahead of this host",
      detail: "The connection remains usable, but newer daemon semantics may be partially unavailable.",
    });
  }

  if (info.assessment.status === "protocol_major_unsupported") {
    reasons.push({
      code: "protocol_major_unsupported",
      scope: "daemon",
      severity: "error",
      summary: "Daemon protocol is incompatible",
      detail: "This host cannot safely assume canonical behavior against the current daemon version.",
    });
  }

  return reasons;
}

export function buildBackendDegradedSemantics(
  info: Omit<TerminalBackendCapabilitiesInfo, "degradedSemantics">,
): TerminalDegradedReason[] {
  const reasons: TerminalDegradedReason[] = [];

  if (info.backend !== "native") {
    reasons.push({
      code: "foreign_backend_projection",
      scope: "backend",
      severity: "warning",
      summary: `Foreign backend - ${info.backend}`,
      detail: "Topology, focus, and viewport semantics are conservative projections rather than canonical native truth.",
    });
  }

  if (!info.capabilities.raw_output_stream) {
    reasons.push({
      code: "raw_output_unavailable",
      scope: "backend",
      severity: "info",
      summary: "Raw output stream is unavailable",
      detail: "Only higher-level projections are available for this backend surface.",
    });
  }

  if (!info.capabilities.rendered_scrollback_snapshot) {
    reasons.push({
      code: "scrollback_snapshot_unavailable",
      scope: "backend",
      severity: "info",
      summary: "Rendered scrollback snapshot is unavailable",
      detail: "This backend cannot provide full rendered scrollback history on demand.",
    });
  }

  return reasons;
}

export function buildSessionDegradedSemantics(
  session: Pick<TerminalSessionSummary, "origin">,
): TerminalDegradedReason[] {
  if (session.origin.backend === "native" && session.origin.authority === "local_daemon") {
    return [];
  }

  return [
    {
      code: "foreign_session_semantics",
      scope: "session",
      severity: "warning",
      summary: `Session uses ${session.origin.backend} semantics`,
      detail: "Canonical IDs are stable, but behavior such as focus, layout, or resize may differ from the native backend.",
    },
  ];
}

export function buildSavedSessionDegradedSemantics(
  session: Omit<TerminalSavedSessionSummary, "degradedSemantics">,
): TerminalDegradedReason[] {
  const reasons = [...buildSessionDegradedSemantics({ origin: session.origin })];

  if (!session.compatibility.can_restore) {
    reasons.push({
      code: "saved_session_restore_unavailable",
      scope: "saved_session",
      severity: "error",
      summary: "Saved session cannot be restored safely",
      detail: `Restore compatibility is ${session.compatibility.status}. The session is intentionally blocked from restore.`,
    });
  }

  if (!session.restore_semantics.preserves_process_state) {
    reasons.push({
      code: "saved_session_process_state_not_preserved",
      scope: "saved_session",
      severity: "warning",
      summary: "Process state is not preserved",
      detail: "Restoring this session recreates topology and launch context, but not the exact running process state.",
    });
  }

  if (!session.restore_semantics.replays_saved_screen_buffers) {
    reasons.push({
      code: "saved_session_screen_buffers_not_replayed",
      scope: "saved_session",
      severity: "info",
      summary: "Screen buffers are not replayed",
      detail: "Viewport history is not reconstructed from the saved snapshot.",
    });
  }

  return reasons;
}

export function buildDiscoveredSessionDegradedSemantics(
  session: Pick<TerminalDiscoveredSession, "backend">,
): TerminalDegradedReason[] {
  if (session.backend === "native") {
    return [];
  }

  return [
    {
      code: "foreign_import_semantics",
      scope: "import",
      severity: "warning",
      summary: `Import from ${session.backend} is conservative`,
      detail: "The imported session keeps canonical IDs, but backend-native behavior remains partial and explicitly non-parity.",
    },
  ];
}

export function findUnsupportedActionDegradedReason(input: {
  action: TerminalWorkspaceActionKind;
  backend: TerminalBackendKind;
  capabilities: TerminalBackendCapabilitiesInfo["capabilities"] | null | undefined;
}): TerminalDegradedReason | null {
  const capabilities = input.capabilities;
  if (!capabilities) {
    return null;
  }

  const capability = actionCapability(input.action);
  if (capabilities[capability]) {
    return null;
  }

  return {
    code: actionCode(input.action),
    scope: "action",
    severity: "warning",
    summary: `${actionLabel(input.action)} is unavailable on ${input.backend}`,
    detail: `This action requires capability \`${capability}\`, which the current backend does not advertise.`,
  };
}

export function findRestoreBlockedReason(
  session: TerminalSavedSessionSummary | null,
): TerminalDegradedReason | null {
  if (!session || session.compatibility.can_restore) {
    return null;
  }

  return session.degradedSemantics.find((reason) => reason.code === "saved_session_restore_unavailable") ?? {
    code: "saved_session_restore_unavailable",
    scope: "saved_session",
    severity: "error",
    summary: "Saved session cannot be restored safely",
    detail: `Restore compatibility is ${session.compatibility.status}.`,
  };
}

function actionCapability(action: TerminalWorkspaceActionKind) {
  switch (action) {
    case "new_tab":
      return "tab_create" as const;
    case "split_pane":
      return "pane_split" as const;
    case "save_session":
      return "explicit_session_save" as const;
    case "focus_pane":
      return "pane_focus" as const;
    case "focus_tab":
      return "tab_focus" as const;
    case "send_input":
      return "pane_input_write" as const;
    default:
      return assertNever(action);
  }
}

function actionLabel(action: TerminalWorkspaceActionKind): string {
  switch (action) {
    case "new_tab":
      return "New tab";
    case "split_pane":
      return "Split pane";
    case "save_session":
      return "Save session";
    case "focus_pane":
      return "Focus pane";
    case "focus_tab":
      return "Focus tab";
    case "send_input":
      return "Pane input";
    default:
      return assertNever(action);
  }
}

function actionCode(action: TerminalWorkspaceActionKind): TerminalDegradedReason["code"] {
  switch (action) {
    case "new_tab":
      return "action_tab_create_unsupported";
    case "split_pane":
      return "action_pane_split_unsupported";
    case "save_session":
      return "action_save_session_unsupported";
    case "focus_pane":
      return "action_pane_focus_unsupported";
    case "focus_tab":
      return "action_tab_focus_unsupported";
    case "send_input":
      return "action_input_write_unsupported";
    default:
      return assertNever(action);
  }
}

function assertNever(value: never): never {
  throw new Error(`Unsupported degraded semantics case: ${JSON.stringify(value)}`);
}
