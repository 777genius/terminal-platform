use std::{
    process::{Command, Stdio},
    sync::{Arc, Mutex as StdMutex},
    thread,
    time::Instant,
};

use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendScope, BackendSessionBinding, BackendSessionPort,
    BackendSessionSummary, BoxFuture, CreateSessionSpec, DiscoveredSession, MuxBackendPort,
};
use terminal_domain::{
    BackendKind, DegradedModeReason, ExternalSessionRef, RouteAuthority, SessionId, SessionRoute,
};
use tokio::{process::Command as TokioCommand, sync::Mutex};

use crate::{
    cli::{
        is_transient_zellij_backend_error, is_transient_zellij_error, zellij_command_path,
        zellij_focus_actions_supported,
    },
    constants::{
        ZELLIJ_COMMAND_POLL_INTERVAL, ZELLIJ_COMMAND_TIMEOUT, ZELLIJ_POLL_INTERVAL,
        ZELLIJ_ROUTE_NAMESPACE, ZELLIJ_TRANSIENT_RETRY_ATTEMPTS,
    },
    probe::{ZellijProbe, ZellijSurface},
    session::ZellijAttachedSession,
    target::ZellijTarget,
};

#[derive(Debug, Clone, Default)]
pub struct ZellijBackend;

impl ZellijBackend {
    #[must_use]
    pub fn kind(&self) -> BackendKind {
        BackendKind::Zellij
    }

    pub(crate) fn run(
        &self,
        target: Option<&ZellijTarget>,
        args: &[&str],
    ) -> Result<String, BackendError> {
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
                return String::from_utf8(output.stdout).map_err(|error| {
                    BackendError::internal(format!("zellij output is not utf8: {error}"))
                });
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
            .arg("--format")
            .arg("json")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        command.spawn().map_err(|error| {
            BackendError::transport(format!("zellij subscribe failed to spawn: {error}"))
        })
    }

    fn probe(&self) -> Result<ZellijProbe, BackendError> {
        let version_output = self.run(None, &["--version"])?;
        let root_help = self.run(None, &["--help"]).ok();
        let action_help = self.run(None, &["action", "--help"]).ok();

        Ok(ZellijProbe::parse(&version_output, root_help.as_deref(), action_help.as_deref()))
    }
}

impl MuxBackendPort for ZellijBackend {
    fn kind(&self) -> BackendKind {
        self.kind()
    }

    fn capabilities(&self) -> BoxFuture<'_, Result<BackendCapabilities, BackendError>> {
        Box::pin(async move {
            let probe = self.probe()?;
            Ok(match probe.surface {
                ZellijSurface::RichCli044Plus => BackendCapabilities {
                    tiled_panes: true,
                    tab_create: true,
                    tab_close: true,
                    tab_focus: zellij_focus_actions_supported(),
                    tab_rename: true,
                    session_scoped_tab_refs: true,
                    session_scoped_pane_refs: true,
                    pane_close: true,
                    pane_focus: zellij_focus_actions_supported(),
                    pane_input_write: true,
                    pane_paste_write: true,
                    rendered_viewport_stream: true,
                    rendered_viewport_snapshot: true,
                    plugin_panes: true,
                    advisory_metadata_subscriptions: true,
                    read_only_client_mode: true,
                    ..BackendCapabilities::default()
                },
                ZellijSurface::LegacyCli043 => BackendCapabilities {
                    read_only_client_mode: true,
                    ..BackendCapabilities::default()
                },
                ZellijSurface::Unknown => BackendCapabilities::default(),
            })
        })
    }

    fn discover_sessions(
        &self,
        _scope: BackendScope,
    ) -> BoxFuture<'_, Result<Vec<DiscoveredSession>, BackendError>> {
        Box::pin(async move {
            let output = match self.run(None, &["list-sessions", "--short", "--no-formatting"]) {
                Ok(output) => output,
                Err(error) if is_transient_zellij_backend_error(&error) => return Ok(Vec::new()),
                Err(error) => return Err(error),
            };
            let sessions = output
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && *line != "No active zellij sessions found.")
                .map(|session_name| {
                    let route = SessionRoute {
                        backend: BackendKind::Zellij,
                        authority: RouteAuthority::ImportedForeign,
                        external: Some(ExternalSessionRef {
                            namespace: ZELLIJ_ROUTE_NAMESPACE.to_string(),
                            value: format!("session={session_name}"),
                        }),
                    };

                    DiscoveredSession { route, title: Some(session_name.to_string()) }
                })
                .collect();

            Ok(sessions)
        })
    }

    fn create_session(
        &self,
        _spec: CreateSessionSpec,
    ) -> BoxFuture<'_, Result<BackendSessionBinding, BackendError>> {
        Box::pin(async {
            Err(BackendError::unsupported(
                "zellij sessions are imported, not created",
                DegradedModeReason::ImportedForeignSession,
            ))
        })
    }

    fn attach_session(
        &self,
        session_id: SessionId,
        route: SessionRoute,
    ) -> BoxFuture<'_, Result<Box<dyn BackendSessionPort>, BackendError>> {
        let backend = self.clone();
        Box::pin(async move {
            let target = ZellijTarget::from_route(&route)?;
            let probe = backend.probe()?;
            let sessions = backend.discover_sessions(BackendScope::CurrentUser).await?;
            if !sessions.iter().any(|session| session.route == route) {
                return Err(BackendError::not_found(format!(
                    "zellij session '{}' is not active",
                    target.session_name
                )));
            }

            match probe.surface {
                ZellijSurface::RichCli044Plus => {
                    let attached = ZellijAttachedSession {
                        backend: Arc::new(backend),
                        session_id,
                        target,
                        io_lane: Arc::new(StdMutex::new(())),
                        command_lane: Arc::new(Mutex::new(())),
                    };
                    attached.snapshot()?;

                    Ok(Box::new(attached) as Box<dyn BackendSessionPort>)
                }
                ZellijSurface::LegacyCli043 => Err(BackendError::unsupported(
                    format!(
                        "zellij {} does not expose the list-panes/list-tabs/subscribe surface required for imported attach",
                        probe.version
                    ),
                    DegradedModeReason::MissingCapability,
                )),
                ZellijSurface::Unknown => Err(BackendError::unsupported(
                    format!(
                        "zellij {} could not be matched to a supported control surface",
                        probe.version
                    ),
                    DegradedModeReason::MissingCapability,
                )),
            }
        })
    }

    fn list_sessions(
        &self,
        _scope: BackendScope,
    ) -> BoxFuture<'_, Result<Vec<BackendSessionSummary>, BackendError>> {
        Box::pin(async {
            Err(BackendError::unsupported(
                "zellij backend does not expose canonical sessions directly",
                DegradedModeReason::ImportedForeignSession,
            ))
        })
    }
}
