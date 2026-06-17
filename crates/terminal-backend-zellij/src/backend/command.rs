use std::{
    process::{Command, Stdio},
    thread,
    time::Instant,
};

use terminal_backend_api::BackendError;
use tokio::process::Command as TokioCommand;

use crate::{
    cli::{is_transient_zellij_error, zellij_command_path},
    constants::{
        ZELLIJ_COMMAND_POLL_INTERVAL, ZELLIJ_COMMAND_TIMEOUT, ZELLIJ_POLL_INTERVAL,
        ZELLIJ_TRANSIENT_RETRY_ATTEMPTS,
    },
    probe::ZellijProbe,
    target::ZellijTarget,
};

use super::ZellijBackend;

impl ZellijBackend {
    pub(crate) fn run(
        &self,
        target: Option<&ZellijTarget>,
        args: &[&str],
    ) -> Result<String, BackendError> {
        let stdout = self.run_bytes(target, args)?;
        String::from_utf8(stdout)
            .map_err(|error| BackendError::internal(format!("zellij output is not utf8: {error}")))
    }

    pub(crate) fn run_bytes(
        &self,
        target: Option<&ZellijTarget>,
        args: &[&str],
    ) -> Result<Vec<u8>, BackendError> {
        let mut last_error = None;
        for attempt in 0..ZELLIJ_TRANSIENT_RETRY_ATTEMPTS {
            let mut command = Command::new(zellij_command_path());
            if let Some(target) = target {
                command.arg("--session").arg(&target.session_name);
            }
            command.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());

            let mut child = command.spawn().map_err(|error| {
                BackendError::transport(format!("zellij command failed to spawn: {error}"))
            })?;
            let started = Instant::now();

            let output = loop {
                match child.try_wait() {
                    Ok(Some(_)) => {
                        break child.wait_with_output().map_err(|error| {
                            BackendError::transport(format!(
                                "zellij command output collection failed: {error}"
                            ))
                        })?;
                    }
                    Ok(None) => {
                        if started.elapsed() >= ZELLIJ_COMMAND_TIMEOUT {
                            let _ = child.kill();
                            let _ = child.wait();
                            return Err(BackendError::transport(format!(
                                "zellij command timed out after {} ms: zellij {}",
                                ZELLIJ_COMMAND_TIMEOUT.as_millis(),
                                args.join(" ")
                            )));
                        }
                        thread::sleep(ZELLIJ_COMMAND_POLL_INTERVAL);
                    }
                    Err(error) => {
                        return Err(BackendError::transport(format!(
                            "zellij command wait failed: {error}"
                        )));
                    }
                }
            };
            if output.status.success() {
                return Ok(output.stdout);
            }

            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let error = BackendError::transport(format!("zellij command failed: {stderr}"));
            if attempt + 1 < ZELLIJ_TRANSIENT_RETRY_ATTEMPTS && is_transient_zellij_error(&stderr) {
                last_error = Some(error);
                thread::sleep(ZELLIJ_POLL_INTERVAL);
                continue;
            }
            return Err(error);
        }

        Err(last_error.unwrap_or_else(|| {
            BackendError::transport("zellij command never reached a stable result")
        }))
    }

    pub(crate) fn run_owned(
        &self,
        target: Option<&ZellijTarget>,
        args: &[String],
    ) -> Result<String, BackendError> {
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        self.run(target, &refs)
    }

    pub(crate) fn run_owned_bytes(
        &self,
        target: Option<&ZellijTarget>,
        args: &[String],
    ) -> Result<Vec<u8>, BackendError> {
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        self.run_bytes(target, &refs)
    }

    pub(crate) fn spawn_subscribe(
        &self,
        target: &ZellijTarget,
        pane_ref: &str,
    ) -> Result<tokio::process::Child, BackendError> {
        let mut command = TokioCommand::new(zellij_command_path());
        command
            .arg("--session")
            .arg(&target.session_name)
            .arg("subscribe")
            .arg("--pane-id")
            .arg(pane_ref)
            .arg("--ansi")
            .arg("--format")
            .arg("json")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        command.spawn().map_err(|error| {
            BackendError::transport(format!("zellij subscribe failed to spawn: {error}"))
        })
    }

    pub(super) fn probe(&self) -> Result<ZellijProbe, BackendError> {
        let version_output = self.run(None, &["--version"])?;
        let root_help = self.run(None, &["--help"]).ok();
        let action_help = self.run(None, &["action", "--help"]).ok();
        let dump_screen_help = self.run(None, &["action", "dump-screen", "--help"]).ok();
        let subscribe_help = self.run(None, &["subscribe", "--help"]).ok();

        Ok(ZellijProbe::parse(
            &version_output,
            root_help.as_deref(),
            action_help.as_deref(),
            dump_screen_help.as_deref(),
            subscribe_help.as_deref(),
        ))
    }
}
