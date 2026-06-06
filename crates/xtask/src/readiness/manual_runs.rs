use std::{fs, path::Path};

use crate::{
    manual::{REQUIRED_MANUAL_RUNS, verify_recorded_pass},
    support::assert_value,
};

pub(crate) fn verify_recorded_passes(manual_runs_dir: &Path) -> Result<(), String> {
    let mut has_electron_pass = false;
    let mut has_tmux_pass = false;
    let mut has_windows_zellij_pass = false;

    for entry in fs::read_dir(manual_runs_dir)
        .map_err(|error| format!("failed to read {} - {error}", manual_runs_dir.display()))?
    {
        let entry = entry
            .map_err(|error| format!("failed to read manual run directory entry - {error}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };

        if matches!(name, "README.md" | "_template.md") {
            continue;
        }

        let contents = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {} - {error}", path.display()))?;
        let Some(expectation) = REQUIRED_MANUAL_RUNS
            .iter()
            .find(|expectation| name.starts_with(expectation.file_prefix))
        else {
            return Err(format!(
                "unexpected manual run artifact in strict v1 gate: {}",
                path.display()
            ));
        };

        verify_recorded_pass(&path, name, &contents, *expectation)?;

        match expectation.file_prefix {
            "electron-" => has_electron_pass = true,
            "unix-tmux-" => has_tmux_pass = true,
            "windows-native-zellij-" => has_windows_zellij_pass = true,
            _ => {}
        }
    }

    assert_value(has_electron_pass, "missing recorded Electron embed pass in manual/runs")?;
    assert_value(has_tmux_pass, "missing recorded Unix tmux pass in manual/runs")?;
    assert_value(
        has_windows_zellij_pass,
        "missing recorded Windows Native + Zellij pass in manual/runs",
    )?;

    Ok(())
}
