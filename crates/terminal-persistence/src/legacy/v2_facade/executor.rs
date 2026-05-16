use std::sync::Arc;

use crate::{
    db::executor::PersistenceExecutor,
    v2::{TerminalPersistenceV2, TerminalPersistenceV2Config, TerminalPersistenceV2Error},
};

use super::super::SqliteSessionStore;

impl SqliteSessionStore {
    pub(in crate::legacy) fn with_v2_store_serialized<T>(
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

    pub(in crate::legacy) fn execute_v2_serialized<T>(
        &self,
        operation: impl FnOnce() -> Result<T, TerminalPersistenceV2Error> + Send + 'static,
    ) -> Result<T, TerminalPersistenceV2Error>
    where
        T: Send + 'static,
    {
        let executor = self.v2_executor()?;
        executor.execute(operation)
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

        let executor = Arc::new(PersistenceExecutor::start(
            &self.path,
            TerminalPersistenceV2Config::default(),
        )?);
        *guard = Some(Arc::clone(&executor));
        Ok(executor)
    }
}
