use serde::{Deserialize, Serialize};

use terminal_backend_api::MuxCommandResult;
use terminal_projection::{ScreenDelta, ScreenSnapshot, SessionHealthSnapshot, TopologySnapshot};

use crate::Handshake;

use super::{
    BackendCapabilitiesResponse, CommandHistoryResponse, CreateSessionResponse,
    DeleteSavedSessionResponse, DiscoverSessionsResponse, ImportSessionResponse,
    ListSavedSessionsResponse, ListSessionsResponse, OpenSubscriptionResponse, PaneHistoryResponse,
    PruneSavedSessionsResponse, RestoreSavedSessionResponse, SavedSessionResponse,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResponsePayload {
    Handshake(Handshake),
    CreateSession(CreateSessionResponse),
    ListSessions(ListSessionsResponse),
    ListSavedSessions(ListSavedSessionsResponse),
    DiscoverSessions(DiscoverSessionsResponse),
    BackendCapabilities(BackendCapabilitiesResponse),
    ImportSession(ImportSessionResponse),
    SavedSession(SavedSessionResponse),
    DeleteSavedSession(DeleteSavedSessionResponse),
    PruneSavedSessions(PruneSavedSessionsResponse),
    RestoreSavedSession(RestoreSavedSessionResponse),
    TopologySnapshot(TopologySnapshot),
    SessionHealthSnapshot(SessionHealthSnapshot),
    ScreenSnapshot(ScreenSnapshot),
    ScreenDelta(ScreenDelta),
    PaneHistory(PaneHistoryResponse),
    CommandHistory(CommandHistoryResponse),
    DispatchMuxCommand(MuxCommandResult),
    SubscriptionOpened(OpenSubscriptionResponse),
}
