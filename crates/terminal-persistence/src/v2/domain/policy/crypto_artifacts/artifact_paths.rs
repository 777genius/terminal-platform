use super::super::super::super::*;
use super::super::super::*;

pub(in crate::v2) fn validate_external_artifact_target_ref(
    artifact_ref: &str,
    source_db_path: &Path,
) -> Result<(), TerminalPersistenceV2Error> {
    let Some(target_path) = path_like_artifact_ref(artifact_ref) else {
        return Ok(());
    };
    let source_canonical = source_db_path.canonicalize()?;
    let Some(target_normalized) = normalize_artifact_target_path(&target_path) else {
        return Ok(());
    };
    let forbidden_targets = [
        source_canonical.clone(),
        sqlite_sidecar_path(&source_canonical, "-wal"),
        sqlite_sidecar_path(&source_canonical, "-shm"),
    ];
    if forbidden_targets
        .iter()
        .any(|forbidden| paths_equal_for_platform(forbidden, &target_normalized))
    {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "external artifact ref cannot point at the live database or SQLite sidecar: {}",
            target_normalized.display()
        )));
    }
    Ok(())
}

pub(in crate::v2) fn path_like_artifact_ref(artifact_ref: &str) -> Option<PathBuf> {
    let trimmed = artifact_ref.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.contains("://") {
        return None;
    }
    let path = PathBuf::from(trimmed);
    if path.is_absolute()
        || trimmed.starts_with('.')
        || trimmed.contains('\\')
        || trimmed.contains('/')
        || looks_like_windows_drive_path(trimmed)
    {
        Some(path)
    } else {
        None
    }
}

pub(in crate::v2) fn looks_like_windows_drive_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
        && bytes[0].is_ascii_alphabetic()
}

pub(in crate::v2) fn normalize_artifact_target_path(path: &Path) -> Option<PathBuf> {
    if let Ok(canonical) = path.canonicalize() {
        return Some(canonical);
    }
    let file_name = path.file_name()?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    parent.canonicalize().ok().map(|parent| parent.join(file_name))
}
