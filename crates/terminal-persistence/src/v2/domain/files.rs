use super::super::*;
use super::*;

pub(in crate::v2) fn blake3_hash_file(path: &Path) -> Result<String, TerminalPersistenceV2Error> {
    let mut file = fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

pub(in crate::v2) fn file_len_i64(path: &Path) -> Result<Option<i64>, TerminalPersistenceV2Error> {
    fs::metadata(path).ok().map(|metadata| u64_to_i64(metadata.len(), "file size")).transpose()
}

pub(in crate::v2) fn prepare_vacuum_backup_target(
    source_path: &Path,
    target_path: &Path,
) -> Result<PathBuf, TerminalPersistenceV2Error> {
    let file_name = target_path.file_name().ok_or_else(|| {
        TerminalPersistenceV2Error::InvalidData(format!(
            "backup target must include a file name: {}",
            target_path.display()
        ))
    })?;
    let parent = target_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let source_canonical = source_path.canonicalize()?;
    let target_absolute = parent.canonicalize()?.join(file_name);
    let forbidden_targets = [
        source_canonical.clone(),
        sqlite_sidecar_path(&source_canonical, "-wal"),
        sqlite_sidecar_path(&source_canonical, "-shm"),
    ];
    if forbidden_targets
        .iter()
        .any(|forbidden| paths_equal_for_platform(forbidden, &target_absolute))
    {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "backup target cannot point at the live database or SQLite sidecar: {}",
            target_absolute.display()
        )));
    }
    if target_absolute.exists() {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "backup target already exists: {}",
            target_absolute.display()
        )));
    }

    Ok(target_absolute)
}

pub(in crate::v2) fn paths_equal_for_platform(left: &Path, right: &Path) -> bool {
    #[cfg(windows)]
    {
        left.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(&right.as_os_str().to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

pub(in crate::v2) fn sqlite_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}
