pub(super) use std::time::{Duration, Instant};

#[cfg(any(unix, windows))]
pub(super) use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
pub(super) use std::{process::Command, sync::Arc};

#[cfg(unix)]
pub(super) use terminal_backend_api::MuxBackendPort;
pub(super) use terminal_backend_api::{
    CreateSessionSpec, MuxCommand, NewTabSpec, OverrideLayoutSpec, ResizePaneSpec, SendInputSpec,
    SendPasteSpec, ShellLaunchSpec, SplitPaneSpec, SubscriptionSpec,
};
#[cfg(unix)]
pub(super) use terminal_backend_native::NativeBackend;
#[cfg(unix)]
pub(super) use terminal_backend_tmux::TmuxBackend;
#[cfg(unix)]
pub(super) use terminal_backend_zellij::ZellijBackend;
#[cfg(any(unix, windows))]
pub(super) use terminal_daemon::TerminalDaemon;
pub(super) use terminal_domain::BackendKind;
#[cfg(any(unix, windows))]
pub(super) use terminal_domain::{
    CURRENT_BINARY_VERSION, CURRENT_PROTOCOL_MAJOR, CURRENT_PROTOCOL_MINOR,
    CURRENT_SAVED_SESSION_FORMAT_VERSION, SavedSessionCompatibilityStatus, SavedSessionManifest,
    SessionId, local_native_route,
};
#[cfg(any(unix, windows))]
pub(super) use terminal_domain::{DegradedModeReason, PaneId, TabId};
#[cfg(any(unix, windows))]
pub(super) use terminal_mux_domain::PaneSplit;
#[cfg(any(unix, windows))]
pub(super) use terminal_mux_domain::PaneTreeNode;
#[cfg(any(unix, windows))]
pub(super) use terminal_mux_domain::{SplitDirection, TabSnapshot};
#[cfg(any(unix, windows))]
pub(super) use terminal_persistence::{
    SqliteSessionStore, TerminalPersistenceV2, TerminalPersistenceV2Config,
};
#[cfg(any(unix, windows))]
pub(super) use terminal_projection::{
    ProjectionSource, ScreenDelta, ScreenSnapshot, TopologySnapshot,
};
pub(super) use terminal_protocol::{DaemonPhase, SubscriptionEvent};
#[cfg(unix)]
pub(super) use terminal_runtime::{BackendCatalog, TerminalRuntime};
pub(super) use terminal_testing::{
    ZellijSessionGuard, daemon, daemon_fixture, echo_shell_launch_spec, unique_zellij_session_name,
};
#[cfg(any(unix, windows))]
pub(super) use terminal_testing::{
    daemon_fixture_with_daemon, isolated_daemon, unique_sqlite_path,
};
pub(super) use tokio::time::{sleep, timeout};
