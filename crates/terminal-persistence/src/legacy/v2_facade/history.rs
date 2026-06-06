use crate::v2::{
    CommandHistoryEntryRecord, PaneHistoryHydrationRecord, TerminalPersistenceV2Error,
};

use super::super::{SqliteSessionStore, retry::retry_v2_write};

impl SqliteSessionStore {
    pub fn hydrate_v2_pane_history(
        &self,
        session_id: &str,
        pane_id: &str,
        from_event_seq: Option<i64>,
        max_segments: Option<i64>,
        max_bytes: Option<i64>,
    ) -> Result<PaneHistoryHydrationRecord, TerminalPersistenceV2Error> {
        let session_id = session_id.to_string();
        let pane_id = pane_id.to_string();
        retry_v2_write(|| {
            let session_id = session_id.clone();
            let pane_id = pane_id.clone();
            self.with_v2_worker_connection(move |store, connection| {
                store.hydrate_pane_history_with_connection(
                    connection,
                    &session_id,
                    &pane_id,
                    from_event_seq,
                    max_segments,
                    max_bytes,
                )
            })
        })
    }

    pub fn list_v2_command_history(
        &self,
        session_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<CommandHistoryEntryRecord>, TerminalPersistenceV2Error> {
        let session_id = session_id.map(ToOwned::to_owned);
        retry_v2_write(|| {
            let session_id = session_id.clone();
            self.with_v2_worker_connection(move |store, connection| {
                store.list_command_history_with_connection(connection, session_id.as_deref(), limit)
            })
        })
    }
}
