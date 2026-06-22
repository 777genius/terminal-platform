pub(super) use std::path::PathBuf;

#[cfg(feature = "native-backend")]
pub(super) use terminal_backend_api::{
    CreateSessionSpec, MuxCommand, NewTabSpec, ShellLaunchSpec, SubscriptionSpec,
};
pub(super) use terminal_domain::{
    CURRENT_BINARY_VERSION, CURRENT_PROTOCOL_MAJOR, CURRENT_PROTOCOL_MINOR, OperationId,
};
#[cfg(feature = "native-backend")]
pub(super) use terminal_domain::{
    SavedSessionCompatibilityStatus, SavedSessionManifest, local_native_route,
};
#[cfg(feature = "native-backend")]
pub(super) use terminal_mux_domain::{PaneTreeNode, TabSnapshot};
pub(super) use terminal_persistence::SqliteSessionStore;
#[cfg(feature = "native-backend")]
pub(super) use terminal_projection::TopologySnapshot;
pub(super) use terminal_protocol::{
    HistoryReplayState, RequestEnvelope, RequestPayload, ResponsePayload, RestoreGuaranteeLevel,
};

pub(super) use super::super::TerminalDaemon;
