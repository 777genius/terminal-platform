use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use crate::v2::TerminalPersistenceV2Error;

pub(super) fn connection_init_lock() -> &'static Mutex<()> {
    static LOCK: Mutex<()> = Mutex::new(());
    &LOCK
}

pub(super) fn is_process_initialized(path: &Path) -> bool {
    initialized_databases().lock().map(|initialized| initialized.contains(path)).unwrap_or(false)
}

pub(super) fn mark_process_initialized(path: PathBuf) -> Result<(), TerminalPersistenceV2Error> {
    initialized_databases()
        .lock()
        .map_err(|_| {
            TerminalPersistenceV2Error::InvalidData(
                "terminal persistence init registry poisoned".to_string(),
            )
        })?
        .insert(path);
    Ok(())
}

fn initialized_databases() -> &'static Mutex<HashSet<PathBuf>> {
    static INITIALIZED_DATABASES: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
    INITIALIZED_DATABASES.get_or_init(|| Mutex::new(HashSet::new()))
}
