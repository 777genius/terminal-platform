pub(super) use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

pub(super) use terminal_daemon::spawn_local_socket_server;
pub(super) use terminal_daemon_client::LocalSocketDaemonClient;
pub(super) use terminal_domain::DegradedModeReason;
#[cfg(unix)]
pub(super) use terminal_testing::echo_shell_launch_spec;
#[cfg(unix)]
pub(super) use terminal_testing::{
    TmuxServerGuard, daemon_fixture_with_daemon, tmux_daemon, unique_tmux_session_name,
    unique_tmux_socket_name,
};
pub(super) use terminal_testing::{
    ZellijSessionGuard, daemon, daemon_fixture, unique_socket_address, unique_zellij_session_name,
    wait_for_daemon_ready,
};
pub(super) use tokio::time::{sleep, timeout};

pub(super) use crate::{
    NodeBackendKind, NodeCreateSessionRequest, NodeDiscoveredSession, NodeExternalSessionRef,
    NodeHostClient, NodeMuxCommand, NodeNewTabCommand, NodePaneTreeNode, NodeProjectionSource,
    NodeRenameTabCommand, NodeRouteAuthority, NodeScreenDelta, NodeScreenSnapshot,
    NodeSendInputCommand, NodeSessionRoute, NodeShellLaunchSpec, NodeSubscriptionEvent,
    NodeSubscriptionHandle, NodeSubscriptionSpec, NodeTopologySnapshot,
    export_typescript_bindings_to,
};
