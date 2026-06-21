#[cfg(unix)]
pub(super) use std::thread;
pub(super) use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(unix)]
pub(super) use rusqlite::{Connection, params};
#[cfg(unix)]
pub(super) use terminal_backend_api::SendInputSpec;
#[cfg(unix)]
pub(super) use terminal_backend_api::ShellLaunchSpec;
pub(super) use terminal_backend_api::{
    CreateSessionSpec, MuxCommand, NewTabSpec, SubscriptionSpec,
};
pub(super) use terminal_daemon::{TerminalDaemon, spawn_local_socket_server};
pub(super) use terminal_domain::{BackendKind, CURRENT_BINARY_VERSION};
#[cfg(unix)]
pub(super) use terminal_domain::{
    CURRENT_PROTOCOL_MAJOR, CURRENT_PROTOCOL_MINOR, CURRENT_SAVED_SESSION_FORMAT_VERSION,
    DegradedModeReason, PaneId, SavedSessionCompatibilityStatus, SavedSessionManifest, SessionId,
    TabId, local_native_route,
};
#[cfg(unix)]
pub(super) use terminal_mux_domain::{PaneTreeNode, TabSnapshot};
#[cfg(unix)]
pub(super) use terminal_persistence::SqliteSessionStore;
#[cfg(unix)]
pub(super) use terminal_projection::TopologySnapshot;
pub(super) use terminal_protocol::{
    DaemonCapabilities, DaemonPhase, Handshake, ProtocolVersion, SubscriptionEvent,
};
#[cfg(unix)]
pub(super) use terminal_protocol::{HistoryReplayState, RestoreGuaranteeLevel};
#[cfg(unix)]
pub(super) use tokio::time::sleep;
pub(super) use tokio::time::timeout;

pub(super) use crate::{
    DaemonClientInfo, HandshakeAssessmentStatus, LocalSocketDaemonClient, LocalSocketSubscription,
};
