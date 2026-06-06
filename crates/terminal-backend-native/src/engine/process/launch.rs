use terminal_backend_api::{BackendError, ShellLaunchSpec};

pub(in crate::engine) fn resolve_launch_spec(
    spec: Option<ShellLaunchSpec>,
) -> Result<ShellLaunchSpec, BackendError> {
    match spec {
        Some(spec) if spec.program.trim().is_empty() => {
            Err(BackendError::invalid_input("shell launch program cannot be empty"))
        }
        Some(spec) => Ok(spec),
        None => Ok(default_launch_spec()),
    }
}

#[cfg(unix)]
fn default_launch_spec() -> ShellLaunchSpec {
    let program = std::env::var("SHELL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "/bin/sh".to_string());

    ShellLaunchSpec::new(program)
}

#[cfg(windows)]
fn default_launch_spec() -> ShellLaunchSpec {
    let program = std::env::var("COMSPEC")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "cmd.exe".to_string());

    ShellLaunchSpec::new(program)
}
