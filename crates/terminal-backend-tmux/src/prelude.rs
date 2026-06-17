pub(crate) use std::{
    collections::{HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    process::Command,
    sync::Arc,
};

pub(crate) use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendScope, BackendSessionBinding, BackendSessionPort,
    BackendSessionSummary, BackendSubscription, BackendSubscriptionEvent, BoxFuture,
    CreateSessionSpec, DiscoveredSession, MuxBackendPort, MuxCommand, MuxCommandResult,
    ResizePaneSpec, SendInputSpec, SendPasteSpec, SplitPaneSpec, SubscriptionSpec,
};
pub(crate) use terminal_domain::{
    BackendKind, DegradedModeReason, ExternalSessionRef, PaneId, RouteAuthority, SessionId,
    SessionRoute, TabId,
};
pub(crate) use terminal_mux_domain::{PaneSplit, PaneTreeNode, SplitDirection, TabSnapshot};
pub(crate) use terminal_projection::{
    ProjectionSource, ScreenDelta, ScreenSnapshot, ScreenSurface, TopologySnapshot,
};
pub(crate) use tokio::{
    sync::{mpsc, oneshot},
    time::{self, MissedTickBehavior},
};
pub(crate) use uuid::Uuid;
