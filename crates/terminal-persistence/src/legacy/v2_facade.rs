use std::sync::Arc;

use crate::{
    db::executor::PersistenceExecutor,
    v2::{
        BackendCapabilityReportInput, CommandHistoryEntryRecord, HistoryGapEventInput,
        PaneHistoryHydrationRecord, RestorePlan, ScreenSnapshotEventInput,
        TerminalOutputEventInput, TerminalPersistenceV2, TerminalPersistenceV2Config,
        TerminalPersistenceV2Error, TopologySnapshotEventInput, UiInputEventInput,
    },
};

use super::{SavedNativeSession, SqliteSessionStore, retry::retry_v2_write};

impl SqliteSessionStore {
    pub fn save_native_session_v2_snapshot(
        &self,
        session: &SavedNativeSession,
    ) -> Result<RestorePlan, TerminalPersistenceV2Error> {
        let session = session.clone();
        retry_v2_write(|| {
            let session = session.clone();
            self.with_v2_store_serialized(move |store| {
                store.import_saved_native_session_snapshot(&session)?;
                store.run_restore_drill(&session.session_id.0.to_string())?;
                store.restore_plan(&session.session_id.0.to_string())
            })
        })
    }

    pub fn native_session_v2_restore_plan(
        &self,
        session_id: terminal_domain::SessionId,
    ) -> Result<Option<RestorePlan>, TerminalPersistenceV2Error> {
        retry_v2_write(|| {
            self.with_v2_store_serialized(move |store| {
                match store.restore_plan(&session_id.0.to_string()) {
                    Ok(plan) => Ok(Some(plan)),
                    Err(TerminalPersistenceV2Error::Query(diesel::result::Error::NotFound)) => {
                        Ok(None)
                    }
                    Err(error) => Err(error),
                }
            })
        })
    }

    pub fn record_v2_backend_capability_report(
        &self,
        input: BackendCapabilityReportInput,
    ) -> Result<String, TerminalPersistenceV2Error> {
        retry_v2_write(|| {
            let input = input.clone();
            self.with_v2_store_serialized(move |store| {
                store.record_backend_capability_report(input)
            })
        })
    }

    pub fn record_v2_ui_input(
        &self,
        input: UiInputEventInput,
    ) -> Result<(), TerminalPersistenceV2Error> {
        retry_v2_write(|| {
            let input = input.clone();
            self.with_v2_store_serialized(move |store| store.record_ui_input_event(input))
        })
    }

    pub fn record_v2_terminal_output(
        &self,
        input: TerminalOutputEventInput,
    ) -> Result<(), TerminalPersistenceV2Error> {
        retry_v2_write(|| {
            let input = input.clone();
            self.with_v2_store_serialized(move |store| {
                store.record_terminal_output_event(input)?;
                Ok(())
            })
        })
    }

    pub fn record_v2_history_gap(
        &self,
        input: HistoryGapEventInput,
    ) -> Result<(), TerminalPersistenceV2Error> {
        retry_v2_write(|| {
            let input = input.clone();
            self.with_v2_store_serialized(move |store| {
                store.record_history_gap_event(input)?;
                Ok(())
            })
        })
    }

    pub fn record_v2_screen_snapshot(
        &self,
        input: ScreenSnapshotEventInput,
    ) -> Result<(), TerminalPersistenceV2Error> {
        retry_v2_write(|| {
            let input = input.clone();
            self.with_v2_store_serialized(move |store| {
                store.record_screen_snapshot_event(input)?;
                Ok(())
            })
        })
    }

    pub fn record_v2_topology_snapshot(
        &self,
        input: TopologySnapshotEventInput,
    ) -> Result<(), TerminalPersistenceV2Error> {
        retry_v2_write(|| {
            let input = input.clone();
            self.with_v2_store_serialized(move |store| {
                store.record_topology_snapshot_event(input)?;
                Ok(())
            })
        })
    }

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
            self.with_v2_store_serialized(move |store| {
                store.hydrate_pane_history(
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
            self.with_v2_store_serialized(move |store| {
                store.list_command_history(session_id.as_deref(), limit)
            })
        })
    }

    fn with_v2_store_serialized<T>(
        &self,
        operation: impl FnOnce(TerminalPersistenceV2) -> Result<T, TerminalPersistenceV2Error>
        + Send
        + 'static,
    ) -> Result<T, TerminalPersistenceV2Error>
    where
        T: Send + 'static,
    {
        let path = self.path.clone();
        self.execute_v2_serialized(move || {
            let store = TerminalPersistenceV2::open_with_config(
                &path,
                TerminalPersistenceV2Config::default(),
            )?;
            operation(store)
        })
    }

    fn execute_v2_serialized<T>(
        &self,
        operation: impl FnOnce() -> Result<T, TerminalPersistenceV2Error> + Send + 'static,
    ) -> Result<T, TerminalPersistenceV2Error>
    where
        T: Send + 'static,
    {
        let executor = self.v2_executor()?;
        executor.execute(move |_connection| operation())
    }

    fn v2_executor(&self) -> Result<Arc<PersistenceExecutor>, TerminalPersistenceV2Error> {
        let mut guard = self.v2_executor.lock().map_err(|_| {
            TerminalPersistenceV2Error::InvalidData(
                "terminal persistence v2 executor lock poisoned".to_string(),
            )
        })?;
        if let Some(executor) = guard.as_ref() {
            return Ok(Arc::clone(executor));
        }

        let executor = Arc::new(PersistenceExecutor::start(
            &self.path,
            TerminalPersistenceV2Config::default(),
        )?);
        *guard = Some(Arc::clone(&executor));
        Ok(executor)
    }
}
