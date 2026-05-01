use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn export_sdk_runtime_types(out_dir: &Path) -> Result<PathBuf, String> {
    if out_dir.exists() {
        fs::remove_dir_all(out_dir)
            .map_err(|error| format!("failed to clean {} - {error}", out_dir.display()))?;
    }

    fs::create_dir_all(out_dir)
        .map_err(|error| format!("failed to create {} - {error}", out_dir.display()))?;

    terminal_node::export_typescript_bindings_to(out_dir).map_err(|error| {
        format!("failed to export runtime types to {} - {error}", out_dir.display())
    })?;

    Ok(out_dir.to_path_buf())
}
