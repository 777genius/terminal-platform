pub mod envelope;
pub mod errors;
pub mod handshake;
pub mod requests;
pub mod responses;
pub mod subscriptions;
pub mod transport;

pub use envelope::{RequestEnvelope, ResponseEnvelope, SubscriptionEnvelope};
pub use errors::ProtocolError;
pub use handshake::{DaemonPhase, Handshake, ProtocolVersion};
pub use requests::{
    CreateSessionRequest, DiscoverSessionsRequest, DispatchMuxCommandRequest,
    GetScreenDeltaRequest, GetScreenSnapshotRequest, GetTopologySnapshotRequest,
    ImportSessionRequest, OpenSubscriptionRequest, RequestPayload,
};
pub use responses::{
    CreateSessionResponse, DiscoverSessionsResponse, ImportSessionResponse, ListSessionsResponse,
    OpenSubscriptionResponse, ResponsePayload,
};
pub use subscriptions::{SubscriptionEvent, SubscriptionRequest, SubscriptionRequestEnvelope};
pub use transport::{LocalSocketAddress, TransportResponse, decode_json_frame, encode_json_frame};
