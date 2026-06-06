use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
};

pub(crate) fn section_between<'a>(contents: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let (_, tail) = contents.split_once(start)?;
    let (section, _) = tail.split_once(end)?;
    Some(section)
}

pub(crate) fn contains_unresolved_manual_placeholder(contents: &str) -> bool {
    let lowered = contents.to_ascii_lowercase();
    ["fill from", "fill after", "placeholder", "yyyy-mm-dd", "1.xx.x", "vxx.x.x"]
        .iter()
        .any(|marker| lowered.contains(marker))
}

pub(crate) fn assert_contains_all(
    contents: &str,
    label: &str,
    needles: &[&str],
) -> Result<(), String> {
    for needle in needles {
        assert_value(
            contents.contains(needle),
            &format!("{label} is missing required marker: {needle}"),
        )?;
    }
    Ok(())
}

pub(crate) fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("xtask workspace root should resolve")
        .to_path_buf()
}

pub(crate) fn detect_os_label() -> String {
    match env::consts::OS {
        "macos" => "macOS".to_string(),
        "linux" => "Linux".to_string(),
        "windows" => "Windows".to_string(),
        other => other.to_string(),
    }
}

pub(crate) fn probe_command(command: &[&str]) -> Option<String> {
    let (program, args) = command.split_first()?;
    let output = ProcessCommand::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

#[cfg(target_os = "linux")]
pub(crate) fn detect_linux_libc() -> Option<String> {
    if cfg!(target_env = "musl") { Some("musl".to_string()) } else { Some("gnu".to_string()) }
}

pub(crate) fn copy_file(source: &Path, target: &Path) -> Result<(), String> {
    fs::copy(source, target).map(|_| ()).map_err(|error| {
        format!("failed to copy {} to {} - {error}", source.display(), target.display())
    })
}

pub(crate) fn copy_file_ensuring_parent(source: &Path, target: &Path) -> Result<(), String> {
    let parent = target
        .parent()
        .ok_or_else(|| format!("target {} does not have a parent directory", target.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {} - {error}", parent.display()))?;
    copy_file(source, target)
}

pub(crate) fn file_name(path: &Path) -> Result<&str, String> {
    path.file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("failed to resolve file name for {}", path.display()))
}

pub(crate) fn write_json(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    let payload = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to serialize {} - {error}", path.display()))?;
    fs::write(path, format!("{payload}\n"))
        .map_err(|error| format!("failed to write {} - {error}", path.display()))
}

pub(crate) fn read_json(path: &Path) -> Result<serde_json::Value, String> {
    let payload = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {} - {error}", path.display()))?;
    serde_json::from_str(&payload)
        .map_err(|error| format!("failed to parse {} - {error}", path.display()))
}

pub(crate) fn assert_value(value: bool, message: &str) -> Result<(), String> {
    if value { Ok(()) } else { Err(message.to_string()) }
}
