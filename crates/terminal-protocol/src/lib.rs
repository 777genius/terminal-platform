pub mod envelope;
pub mod errors;
pub mod handshake;
pub mod requests;
pub mod responses;
pub mod subscriptions;
pub mod transport;

pub use envelope::{RequestEnvelope, ResponseEnvelope, SubscriptionEnvelope};
pub use errors::ProtocolError;
pub use handshake::{DaemonCapabilities, DaemonPhase, Handshake, ProtocolVersion};
pub use requests::{
    CreateSessionRequest, DeleteSavedSessionRequest, DiscoverSessionsRequest,
    DispatchMuxCommandRequest, GetBackendCapabilitiesRequest, GetSavedSessionRequest,
    GetScreenDeltaRequest, GetScreenSnapshotRequest, GetTopologySnapshotRequest,
    ImportSessionRequest, OpenSubscriptionRequest, PruneSavedSessionsRequest, RequestPayload,
    RestoreSavedSessionRequest,
};
pub use responses::{
    BackendCapabilitiesResponse, CreateSessionResponse, DeleteSavedSessionResponse,
    DiscoverSessionsResponse, ImportSessionResponse, ListSavedSessionsResponse,
    ListSessionsResponse, OpenSubscriptionResponse, PruneSavedSessionsResponse, ResponsePayload,
    RestoreSavedSessionResponse, SavedSessionRecord, SavedSessionResponse,
    SavedSessionRestoreSemantics, SavedSessionSummary,
};
pub use subscriptions::{SubscriptionEvent, SubscriptionRequest, SubscriptionRequestEnvelope};
pub use transport::{LocalSocketAddress, TransportResponse, decode_json_frame, encode_json_frame};
