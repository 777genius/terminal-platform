use super::{prelude::*, *};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum NodeSubscriptionSpec {
    SessionTopology,
    PaneSurface { pane_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeSessionHealthPhase {
    Ready,
    Degraded,
    Stale,
    Terminated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeSessionHealthReason {
    BackendDegraded,
    SubscriptionSourceClosed,
    SessionNotFound,
    BackendTransportLost,
    BackendInternalFault,
    HistoryPersistenceFault,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeSessionHealthSnapshot {
    pub session_id: String,
    pub phase: NodeSessionHealthPhase,
    pub can_attach: bool,
    pub invalidated: bool,
    pub reason: Option<NodeSessionHealthReason>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum NodeSubscriptionEvent {
    TopologySnapshot(NodeTopologySnapshot),
    ScreenDelta(NodeScreenDelta),
    SessionHealthSnapshot(NodeSessionHealthSnapshot),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeSubscriptionMeta {
    pub subscription_id: String,
}
