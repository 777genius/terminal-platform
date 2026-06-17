use crate::{prelude::*, target::TmuxTarget};

use super::TmuxBackend;

impl TmuxBackend {
    pub(crate) fn run(
        &self,
        target: Option<&TmuxTarget>,
        args: &[&str],
    ) -> Result<String, BackendError> {
        let stdout = self.run_bytes(target, args)?;
        String::from_utf8(stdout)
            .map_err(|error| BackendError::internal(format!("tmux output is not utf8: {error}")))
    }

    pub(crate) fn run_bytes(
        &self,
        target: Option<&TmuxTarget>,
        args: &[&str],
    ) -> Result<Vec<u8>, BackendError> {
        let mut command = Command::new("tmux");
        if let Some(socket_name) =
            target.and_then(|target| target.socket_name.as_deref()).or(self.socket_name.as_deref())
        {
            command.arg("-L").arg(socket_name);
        }
        command.args(args);

        let output = command.output().map_err(|error| {
            BackendError::transport(format!("tmux command failed to spawn: {error}"))
        })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BackendError::transport(format!("tmux command failed: {}", stderr.trim())));
        }

        Ok(output.stdout)
    }

    pub(crate) fn run_owned(
        &self,
        target: Option<&TmuxTarget>,
        args: &[String],
    ) -> Result<String, BackendError> {
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        self.run(target, &refs)
    }

    pub(crate) fn is_no_server_running_error(error: &BackendError) -> bool {
        if error.kind != terminal_backend_api::BackendErrorKind::Transport {
            return false;
        }

        let message = error.message.to_ascii_lowercase();
        message.contains("no server running on")
            || (message.contains("error connecting to")
                && message.contains("no such file or directory"))
    }
}
