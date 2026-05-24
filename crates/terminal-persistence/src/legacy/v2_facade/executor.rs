use std::sync::Arc;

use diesel::sqlite::SqliteConnection;

use crate::{
    db::executor::PersistenceExecutor,
    v2::{TerminalPersistenceV2, TerminalPersistenceV2Error},
};

use super::super::SqliteSessionStore;

impl SqliteSessionStore {
    pub(in crate::legacy) fn with_v2_worker_connection<T>(
        &self,
        operation: impl FnOnce(
            &TerminalPersistenceV2,
            &mut SqliteConnection,
        ) -> Result<T, TerminalPersistenceV2Error>
        + Send
        + 'static,
    ) -> Result<T, TerminalPersistenceV2Error>
    where
        T: Send + 'static,
    {
        let path = self.path.clone();
        let config = self.v2_config.clone();
        let executor = self.v2_executor()?;
        executor.execute_with_connection(move |connection| {
            let store = TerminalPersistenceV2::worker_view(path, config);
            operation(&store, connection)
        })
    }

    pub(in crate::legacy) fn v2_executor(
        &self,
    ) -> Result<Arc<PersistenceExecutor>, TerminalPersistenceV2Error> {
        let mut guard = self.v2_executor.lock().map_err(|_| {
            TerminalPersistenceV2Error::InvalidData(
                "terminal persistence v2 executor lock poisoned".to_string(),
            )
        })?;
        if let Some(executor) = guard.as_ref() {
            return Ok(Arc::clone(executor));
        }

        let executor = Arc::new(PersistenceExecutor::start(&self.path, self.v2_config.clone())?);
        *guard = Some(Arc::clone(&executor));
        Ok(executor)
    }
}
