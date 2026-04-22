export type TerminalBackendKind = "native" | "tmux" | "zellij";

export type TerminalRouteAuthority = "local_daemon" | "imported_foreign";

export type TerminalDegradedSeverity = "info" | "warning" | "error";

export type TerminalDegradedScope =
  | "daemon"
  | "backend"
  | "session"
  | "saved_session"
  | "import"
  | "action";

export type TerminalDegradedReasonCode =
  | "daemon_degraded"
  | "daemon_starting"
  | "protocol_minor_ahead"
  | "protocol_major_unsupported"
  | "foreign_backend_projection"
  | "raw_output_unavailable"
  | "scrollback_snapshot_unavailable"
  | "foreign_session_semantics"
  | "saved_session_restore_unavailable"
  | "saved_session_process_state_not_preserved"
  | "saved_session_screen_buffers_not_replayed"
  | "foreign_import_semantics"
  | "action_tab_create_unsupported"
  | "action_pane_split_unsupported"
  | "action_save_session_unsupported"
  | "action_pane_focus_unsupported"
  | "action_tab_focus_unsupported"
  | "action_input_write_unsupported";

export interface TerminalDegradedReason {
  code: TerminalDegradedReasonCode;
  scope: TerminalDegradedScope;
  severity: TerminalDegradedSeverity;
  summary: string;
  detail: string;
}

export interface TerminalSessionOrigin {
  backend: TerminalBackendKind;
  authority: TerminalRouteAuthority;
  foreignReferenceLabel: string | null;
}

export interface TerminalSessionSummary {
  session_id: string;
  origin: TerminalSessionOrigin;
  title: string | null;
  degradedSemantics: TerminalDegradedReason[];
}

export interface TerminalSavedSessionManifest {
  format_version: number;
  binary_version: string;
  protocol_major: number;
  protocol_minor: number;
}

export type TerminalSavedSessionCompatibilityStatus =
  | "compatible"
  | "binary_skew"
  | "format_version_unsupported"
  | "protocol_major_unsupported"
  | "protocol_minor_ahead";

export interface TerminalSavedSessionCompatibility {
  can_restore: boolean;
  status: TerminalSavedSessionCompatibilityStatus;
}

export interface TerminalSavedSessionRestoreSemantics {
  restores_topology: boolean;
  restores_focus_state: boolean;
  restores_tab_titles: boolean;
  uses_saved_launch_spec: boolean;
  replays_saved_screen_buffers: boolean;
  preserves_process_state: boolean;
}

export interface TerminalSavedSessionSummary {
  session_id: string;
  origin: TerminalSessionOrigin;
  title: string | null;
  saved_at_ms: number;
  manifest: TerminalSavedSessionManifest;
  compatibility: TerminalSavedSessionCompatibility;
  has_launch: boolean;
  tab_count: number;
  pane_count: number;
  restore_semantics: TerminalSavedSessionRestoreSemantics;
  degradedSemantics: TerminalDegradedReason[];
}

export interface TerminalDiscoveredSession {
  importHandle: string;
  backend: TerminalBackendKind;
  title: string | null;
  sourceLabel: string;
  degradedSemantics: TerminalDegradedReason[];
}

export interface TerminalBackendCapabilities {
  tiled_panes: boolean;
  floating_panes: boolean;
  split_resize: boolean;
  tab_create: boolean;
  tab_close: boolean;
  tab_focus: boolean;
  tab_rename: boolean;
  session_scoped_tab_refs: boolean;
  session_scoped_pane_refs: boolean;
  pane_split: boolean;
  pane_close: boolean;
  pane_focus: boolean;
  pane_input_write: boolean;
  pane_paste_write: boolean;
  raw_output_stream: boolean;
  rendered_viewport_stream: boolean;
  rendered_viewport_snapshot: boolean;
  rendered_scrollback_snapshot: boolean;
  layout_dump: boolean;
  layout_override: boolean;
  read_only_client_mode: boolean;
  explicit_session_save: boolean;
  explicit_session_restore: boolean;
  plugin_panes: boolean;
  advisory_metadata_subscriptions: boolean;
  independent_resize_authority: boolean;
}

export interface TerminalBackendCapabilitiesInfo {
  backend: TerminalBackendKind;
  capabilities: TerminalBackendCapabilities;
  degradedSemantics: TerminalDegradedReason[];
}

export interface TerminalProtocolVersion {
  major: number;
  minor: number;
}

export type TerminalDaemonPhase = "starting" | "ready" | "degraded";

export interface TerminalDaemonCapabilities {
  request_reply: boolean;
  topology_subscriptions: boolean;
  pane_subscriptions: boolean;
  backend_discovery: boolean;
  backend_capability_queries: boolean;
  saved_sessions: boolean;
  session_restore: boolean;
  degraded_error_reasons: boolean;
}

export interface TerminalHandshake {
  protocol_version: TerminalProtocolVersion;
  binary_version: string;
  daemon_phase: TerminalDaemonPhase;
  capabilities: TerminalDaemonCapabilities;
  available_backends: TerminalBackendKind[];
  session_scope: string;
}

export type TerminalProtocolCompatibilityStatus =
  | "compatible"
  | "protocol_major_unsupported"
  | "protocol_minor_ahead";

export interface TerminalProtocolCompatibility {
  can_connect: boolean;
  status: TerminalProtocolCompatibilityStatus;
}

export type TerminalHandshakeAssessmentStatus =
  | "ready"
  | "starting"
  | "degraded"
  | "protocol_major_unsupported"
  | "protocol_minor_ahead";

export interface TerminalHandshakeAssessment {
  can_use: boolean;
  protocol: TerminalProtocolCompatibility;
  status: TerminalHandshakeAssessmentStatus;
}

export interface TerminalHandshakeInfo {
  handshake: TerminalHandshake;
  assessment: TerminalHandshakeAssessment;
  degradedSemantics: TerminalDegradedReason[];
}

export type TerminalProjectionSource =
  | "native_emulator"
  | "native_transcript"
  | "tmux_capture_pane"
  | "tmux_raw_output_import"
  | "zellij_viewport_subscribe"
  | "zellij_dump_snapshot";

export interface TerminalScreenCursor {
  row: number;
  col: number;
}

export interface TerminalScreenLine {
  text: string;
}

export interface TerminalScreenSurface {
  title: string | null;
  cursor: TerminalScreenCursor | null;
  lines: TerminalScreenLine[];
}

export interface TerminalScreenSnapshot {
  pane_id: string;
  sequence: string;
  rows: number;
  cols: number;
  source: TerminalProjectionSource;
  surface: TerminalScreenSurface;
}

export type TerminalSplitDirection = "horizontal" | "vertical";

export type TerminalPaneTreeNode =
  | {
      kind: "leaf";
      pane_id: string;
    }
  | {
      kind: "split";
      direction: TerminalSplitDirection;
      first: TerminalPaneTreeNode;
      second: TerminalPaneTreeNode;
    };

export interface TerminalTabSnapshot {
  tab_id: string;
  title: string | null;
  root: TerminalPaneTreeNode;
  focused_pane: string | null;
}

export interface TerminalTopologySnapshot {
  session_id: string;
  backend_kind: TerminalBackendKind;
  tabs: TerminalTabSnapshot[];
  focused_tab: string | null;
}

export interface TerminalSessionState {
  session: TerminalSessionSummary;
  topology: TerminalTopologySnapshot;
  focusedScreen: TerminalScreenSnapshot | null;
}

export interface TerminalShellLaunchSpec {
  program: string;
  args: string[];
  cwd?: string;
}

export interface TerminalCreateNativeSessionInput {
  title?: string;
  launch?: TerminalShellLaunchSpec;
}

export interface TerminalImportSessionInput {
  importHandle: string;
  title?: string;
}

export interface TerminalDeleteSavedSessionResponse {
  sessionId: string;
}

export interface TerminalMuxCommandResult {
  changed: boolean;
}

export type TerminalMuxCommand =
  | {
      kind: "split_pane";
      pane_id: string;
      direction: TerminalSplitDirection;
    }
  | {
      kind: "close_pane";
      pane_id: string;
    }
  | {
      kind: "focus_pane";
      pane_id: string;
    }
  | {
      kind: "resize_pane";
      pane_id: string;
      rows: number;
      cols: number;
    }
  | {
      kind: "new_tab";
      title: string | null;
    }
  | {
      kind: "close_tab";
      tab_id: string;
    }
  | {
      kind: "focus_tab";
      tab_id: string;
    }
  | {
      kind: "rename_tab";
      tab_id: string;
      title: string;
    }
  | {
      kind: "send_input";
      pane_id: string;
      data: string;
    }
  | {
      kind: "send_paste";
      pane_id: string;
      data: string;
    }
  | {
      kind: "detach";
    }
  | {
      kind: "save_session";
    }
  | {
      kind: "override_layout";
      tab_id: string;
      root: TerminalPaneTreeNode;
    };
