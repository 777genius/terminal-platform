use super::super::*;

pub fn shell_metadata_profile(
    launch: Option<&ShellLaunchSpec>,
    explicit_shell_kind: Option<&str>,
) -> ShellMetadataProfile {
    let shell_kind = explicit_shell_kind
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim().to_ascii_lowercase())
        .or_else(|| launch.and_then(|launch| infer_shell_kind_from_program(&launch.program)));
    let windows_profile = matches!(shell_kind.as_deref(), Some("cmd" | "powershell" | "pwsh"));
    let command_boundary_confidence = match shell_kind.as_deref() {
        Some("cmd" | "powershell" | "pwsh") => "high",
        Some("bash" | "sh" | "zsh" | "fish") => "high",
        Some(_) => "medium",
        None => "unknown",
    }
    .to_string();
    let cwd_source = if launch.and_then(|launch| launch.cwd.as_ref()).is_some() {
        "launch_cwd"
    } else {
        "unknown"
    }
    .to_string();
    let input_terminator = if windows_profile { "cr" } else { "lf_or_cr" }.to_string();

    ShellMetadataProfile {
        shell_kind,
        command_boundary_confidence,
        cwd_source,
        input_terminator,
        windows_profile,
    }
}

pub(in crate::v2) fn infer_shell_kind_from_program(program: &str) -> Option<String> {
    let normalized = program.replace('\\', "/");
    let file_name = normalized.rsplit('/').next().unwrap_or(program).to_ascii_lowercase();
    let stem = file_name.strip_suffix(".exe").unwrap_or(&file_name);
    match stem {
        "cmd" | "cmd32" | "cmd64" => Some("cmd".to_string()),
        "powershell" => Some("powershell".to_string()),
        "pwsh" => Some("pwsh".to_string()),
        "bash" => Some("bash".to_string()),
        "sh" => Some("sh".to_string()),
        "zsh" => Some("zsh".to_string()),
        "fish" => Some("fish".to_string()),
        _ => None,
    }
}
