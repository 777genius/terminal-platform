use std::path::{Path, PathBuf};

use crate::v2::TerminalPersistenceV2Error;

pub(super) fn path_to_database_url(path: &Path) -> Result<String, TerminalPersistenceV2Error> {
    path.canonicalize()
        .or_else(|_| Ok::<PathBuf, std::io::Error>(path.to_path_buf()))?
        .to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            TerminalPersistenceV2Error::InvalidData("database path is not UTF-8".to_string())
        })
}

pub(super) fn database_init_key(path: &Path) -> Result<PathBuf, TerminalPersistenceV2Error> {
    path.canonicalize()
        .or_else(|_| Ok::<PathBuf, std::io::Error>(path.to_path_buf()))
        .map_err(Into::into)
}
