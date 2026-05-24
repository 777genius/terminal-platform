use crate::v2::{RestorePlan, TerminalPersistenceV2Error};

use super::super::{SavedNativeSession, SqliteSessionStore, retry::retry_v2_write};

impl SqliteSessionStore {
    pub fn save_native_session_v2_snapshot(
        &self,
        session: &SavedNativeSession,
    ) -> Result<RestorePlan, TerminalPersistenceV2Error> {
        let session = session.clone();
        retry_v2_write(|| {
            let session = session.clone();
            self.with_v2_worker_connection(move |store, connection| {
                store.import_saved_native_session_snapshot_with_connection(connection, &session)?;
                store.run_restore_drill_with_connection(
                    connection,
                    &session.session_id.0.to_string(),
                )?;
                store.restore_plan_with_connection(connection, &session.session_id.0.to_string())
            })
        })
    }

    pub fn native_session_v2_restore_plan(
        &self,
        session_id: terminal_domain::SessionId,
    ) -> Result<Option<RestorePlan>, TerminalPersistenceV2Error> {
        retry_v2_write(|| {
            self.with_v2_worker_connection(move |store, connection| {
                match store.restore_plan_with_connection(connection, &session_id.0.to_string()) {
                    Ok(plan) => Ok(Some(plan)),
                    Err(TerminalPersistenceV2Error::Query(diesel::result::Error::NotFound)) => {
                        Ok(None)
                    }
                    Err(error) => Err(error),
                }
            })
        })
    }
}
