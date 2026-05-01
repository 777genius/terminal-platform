use super::{prelude::*, *};

impl NodeBindingVersion {
    #[must_use]
    pub fn current(protocol: &ProtocolVersion) -> Self {
        Self { binding_version: env!("CARGO_PKG_VERSION").to_string(), protocol: protocol.into() }
    }
}

impl From<&ProtocolVersion> for NodeProtocolVersion {
    fn from(value: &ProtocolVersion) -> Self {
        Self { major: value.major, minor: value.minor }
    }
}

impl From<&ProtocolCompatibility> for NodeProtocolCompatibility {
    fn from(value: &ProtocolCompatibility) -> Self {
        Self { can_connect: value.can_connect, status: (&value.status).into() }
    }
}

impl From<&ProtocolCompatibilityStatus> for NodeProtocolCompatibilityStatus {
    fn from(value: &ProtocolCompatibilityStatus) -> Self {
        match value {
            ProtocolCompatibilityStatus::Compatible => Self::Compatible,
            ProtocolCompatibilityStatus::ProtocolMajorUnsupported => Self::ProtocolMajorUnsupported,
            ProtocolCompatibilityStatus::ProtocolMinorAhead => Self::ProtocolMinorAhead,
        }
    }
}

impl From<&HandshakeAssessment> for NodeHandshakeAssessment {
    fn from(value: &HandshakeAssessment) -> Self {
        Self {
            can_use: value.can_use,
            protocol: (&value.protocol).into(),
            status: (&value.status).into(),
        }
    }
}

impl From<&HandshakeAssessmentStatus> for NodeHandshakeAssessmentStatus {
    fn from(value: &HandshakeAssessmentStatus) -> Self {
        match value {
            HandshakeAssessmentStatus::Ready => Self::Ready,
            HandshakeAssessmentStatus::Starting => Self::Starting,
            HandshakeAssessmentStatus::Degraded => Self::Degraded,
            HandshakeAssessmentStatus::ProtocolMajorUnsupported => Self::ProtocolMajorUnsupported,
            HandshakeAssessmentStatus::ProtocolMinorAhead => Self::ProtocolMinorAhead,
        }
    }
}

impl From<&Handshake> for NodeHandshake {
    fn from(value: &Handshake) -> Self {
        Self {
            protocol_version: (&value.protocol_version).into(),
            binary_version: value.binary_version.clone(),
            daemon_phase: (&value.daemon_phase).into(),
            capabilities: (&value.capabilities).into(),
            available_backends: value.available_backends.iter().map(Into::into).collect(),
            session_scope: value.session_scope.clone(),
        }
    }
}

impl From<&DaemonPhase> for NodeDaemonPhase {
    fn from(value: &DaemonPhase) -> Self {
        match value {
            DaemonPhase::Starting => Self::Starting,
            DaemonPhase::Ready => Self::Ready,
            DaemonPhase::Degraded => Self::Degraded,
        }
    }
}

impl From<&DaemonCapabilities> for NodeDaemonCapabilities {
    fn from(value: &DaemonCapabilities) -> Self {
        Self {
            request_reply: value.request_reply,
            topology_subscriptions: value.topology_subscriptions,
            pane_subscriptions: value.pane_subscriptions,
            backend_discovery: value.backend_discovery,
            backend_capability_queries: value.backend_capability_queries,
            saved_sessions: value.saved_sessions,
            session_restore: value.session_restore,
            degraded_error_reasons: value.degraded_error_reasons,
            session_health: value.session_health,
        }
    }
}

impl From<&SessionHealthPhase> for NodeSessionHealthPhase {
    fn from(value: &SessionHealthPhase) -> Self {
        match value {
            SessionHealthPhase::Ready => Self::Ready,
            SessionHealthPhase::Degraded => Self::Degraded,
            SessionHealthPhase::Stale => Self::Stale,
            SessionHealthPhase::Terminated => Self::Terminated,
        }
    }
}

impl From<&SessionHealthReason> for NodeSessionHealthReason {
    fn from(value: &SessionHealthReason) -> Self {
        match value {
            SessionHealthReason::BackendDegraded => Self::BackendDegraded,
            SessionHealthReason::SubscriptionSourceClosed => Self::SubscriptionSourceClosed,
            SessionHealthReason::SessionNotFound => Self::SessionNotFound,
            SessionHealthReason::BackendTransportLost => Self::BackendTransportLost,
            SessionHealthReason::BackendInternalFault => Self::BackendInternalFault,
        }
    }
}

impl From<&SessionHealthSnapshot> for NodeSessionHealthSnapshot {
    fn from(value: &SessionHealthSnapshot) -> Self {
        Self {
            session_id: value.session_id.0.to_string(),
            phase: (&value.phase).into(),
            can_attach: value.can_attach,
            invalidated: value.invalidated,
            reason: value.reason.as_ref().map(Into::into),
            detail: value.detail.clone(),
        }
    }
}

impl From<&BackendKind> for NodeBackendKind {
    fn from(value: &BackendKind) -> Self {
        match value {
            BackendKind::Native => Self::Native,
            BackendKind::Tmux => Self::Tmux,
            BackendKind::Zellij => Self::Zellij,
        }
    }
}

impl From<&RouteAuthority> for NodeRouteAuthority {
    fn from(value: &RouteAuthority) -> Self {
        match value {
            RouteAuthority::LocalDaemon => Self::LocalDaemon,
            RouteAuthority::ImportedForeign => Self::ImportedForeign,
        }
    }
}

impl From<&SessionRoute> for NodeSessionRoute {
    fn from(value: &SessionRoute) -> Self {
        Self {
            backend: (&value.backend).into(),
            authority: (&value.authority).into(),
            external: value.external.as_ref().map(Into::into),
        }
    }
}

impl From<&terminal_domain::ExternalSessionRef> for NodeExternalSessionRef {
    fn from(value: &terminal_domain::ExternalSessionRef) -> Self {
        Self { namespace: value.namespace.clone(), value: value.value.clone() }
    }
}

impl From<&NodeShellLaunchSpec> for ShellLaunchSpec {
    fn from(value: &NodeShellLaunchSpec) -> Self {
        let mut spec = ShellLaunchSpec::new(value.program.clone()).with_args(value.args.clone());
        if let Some(cwd) = &value.cwd {
            spec = spec.with_cwd(cwd);
        }
        spec
    }
}

impl From<&ShellLaunchSpec> for NodeShellLaunchSpec {
    fn from(value: &ShellLaunchSpec) -> Self {
        Self {
            program: value.program.clone(),
            args: value.args.clone(),
            cwd: value.cwd.as_ref().map(|cwd| cwd.display().to_string()),
        }
    }
}

impl From<&NodeCreateSessionRequest> for CreateSessionSpec {
    fn from(value: &NodeCreateSessionRequest) -> Self {
        Self { title: value.title.clone(), launch: value.launch.as_ref().map(Into::into) }
    }
}

impl From<&BackendSessionSummary> for NodeSessionSummary {
    fn from(value: &BackendSessionSummary) -> Self {
        Self {
            session_id: value.session_id.0.to_string(),
            route: (&value.route).into(),
            title: value.title.clone(),
        }
    }
}

impl From<&DiscoveredSession> for NodeDiscoveredSession {
    fn from(value: &DiscoveredSession) -> Self {
        Self { route: (&value.route).into(), title: value.title.clone() }
    }
}

impl From<&BackendCapabilities> for NodeBackendCapabilities {
    fn from(value: &BackendCapabilities) -> Self {
        Self {
            tiled_panes: value.tiled_panes,
            floating_panes: value.floating_panes,
            split_resize: value.split_resize,
            tab_create: value.tab_create,
            tab_close: value.tab_close,
            tab_focus: value.tab_focus,
            tab_rename: value.tab_rename,
            session_scoped_tab_refs: value.session_scoped_tab_refs,
            session_scoped_pane_refs: value.session_scoped_pane_refs,
            pane_split: value.pane_split,
            pane_close: value.pane_close,
            pane_focus: value.pane_focus,
            pane_input_write: value.pane_input_write,
            pane_paste_write: value.pane_paste_write,
            raw_output_stream: value.raw_output_stream,
            rendered_viewport_stream: value.rendered_viewport_stream,
            rendered_viewport_snapshot: value.rendered_viewport_snapshot,
            rendered_scrollback_snapshot: value.rendered_scrollback_snapshot,
            layout_dump: value.layout_dump,
            layout_override: value.layout_override,
            read_only_client_mode: value.read_only_client_mode,
            explicit_session_save: value.explicit_session_save,
            explicit_session_restore: value.explicit_session_restore,
            plugin_panes: value.plugin_panes,
            advisory_metadata_subscriptions: value.advisory_metadata_subscriptions,
            independent_resize_authority: value.independent_resize_authority,
        }
    }
}

impl From<&BackendCapabilitiesResponse> for NodeBackendCapabilitiesInfo {
    fn from(value: &BackendCapabilitiesResponse) -> Self {
        Self { backend: (&value.backend).into(), capabilities: (&value.capabilities).into() }
    }
}
