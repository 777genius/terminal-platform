#[cfg(all(any(unix, windows), feature = "zellij-backend"))]
pub(super) use std::time::Instant;
pub(super) use std::{path::PathBuf, time::Duration};

pub(super) use terminal_daemon::spawn_local_socket_server;
pub(super) use terminal_daemon_client::LocalSocketDaemonClient;
#[cfg(all(any(unix, windows), feature = "zellij-backend"))]
pub(super) use terminal_domain::DegradedModeReason;
#[cfg(unix)]
pub(super) use terminal_testing::echo_shell_launch_spec;
#[cfg(all(unix, feature = "tmux-backend"))]
pub(super) use terminal_testing::{
    TmuxServerGuard, daemon_fixture_with_daemon, tmux_daemon, unique_tmux_session_name,
    unique_tmux_socket_name,
};
#[cfg(all(any(unix, windows), feature = "zellij-backend"))]
pub(super) use terminal_testing::{ZellijSessionGuard, unique_zellij_session_name};
pub(super) use terminal_testing::{
    daemon, daemon_fixture, unique_socket_address, wait_for_daemon_ready,
};
pub(super) use tokio::time::{sleep, timeout};

pub(super) use crate::{
    NodeBackendKind, NodeCreateSessionRequest, NodeHostClient, NodeMuxCommand, NodeNewTabCommand,
    NodeRenameTabCommand, NodeScreenDelta, NodeScreenSnapshot, NodeSendInputCommand,
    NodeShellLaunchSpec, NodeSubscriptionEvent, NodeSubscriptionHandle, NodeSubscriptionSpec,
    NodeTopologySnapshot, export_typescript_bindings_to,
};
#[cfg(all(any(unix, windows), feature = "zellij-backend"))]
pub(super) use crate::{
    NodeDiscoveredSession, NodeExternalSessionRef, NodePaneTreeNode, NodeProjectionSource,
    NodeRouteAuthority, NodeSessionRoute,
};
