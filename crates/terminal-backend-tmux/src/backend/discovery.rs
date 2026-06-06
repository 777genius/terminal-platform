use crate::{prelude::*, target::TmuxTarget};

use super::TmuxBackend;

pub(super) fn discover_tmux_sessions(
    backend: &TmuxBackend,
) -> Result<Vec<DiscoveredSession>, BackendError> {
    let output = match backend.run(
        None,
        &["list-sessions", "-F", "#{session_name}\t#{session_windows}\t#{session_attached}"],
    ) {
        Ok(output) => output,
        Err(error) if TmuxBackend::is_no_server_running_error(&error) => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };

    let sessions = output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let session_name = line.split('\t').next()?;
            let target = TmuxTarget {
                socket_name: backend.socket_name.clone(),
                session_name: session_name.to_string(),
            };
            Some(DiscoveredSession { route: target.route(), title: Some(session_name.to_string()) })
        })
        .collect();

    Ok(sessions)
}
