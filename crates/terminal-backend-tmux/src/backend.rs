use crate::{prelude::*, session::TmuxAttachedSession, target::TmuxTarget};

#[derive(Debug, Clone, Default)]
pub struct TmuxBackend {
    socket_name: Option<String>,
}

impl TmuxBackend {
    #[must_use]
    pub fn with_socket_name(socket_name: impl Into<String>) -> Self {
        Self { socket_name: Some(socket_name.into()) }
    }

    #[must_use]
    pub fn kind(&self) -> BackendKind {
        BackendKind::Tmux
    }

    pub(crate) fn run(
        &self,
        target: Option<&TmuxTarget>,
        args: &[&str],
    ) -> Result<String, BackendError> {
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

        String::from_utf8(output.stdout)
            .map_err(|error| BackendError::internal(format!("tmux output is not utf8: {error}")))
    }

    pub(crate) fn run_owned(
        &self,
        target: Option<&TmuxTarget>,
        args: &[String],
    ) -> Result<String, BackendError> {
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        self.run(target, &refs)
    }

    fn is_no_server_running_error(error: &BackendError) -> bool {
        if error.kind != terminal_backend_api::BackendErrorKind::Transport {
            return false;
        }

        let message = error.message.to_ascii_lowercase();
        message.contains("no server running on")
            || (message.contains("error connecting to")
                && message.contains("no such file or directory"))
    }
}

impl MuxBackendPort for TmuxBackend {
    fn kind(&self) -> BackendKind {
        self.kind()
    }

    fn capabilities(&self) -> BoxFuture<'_, Result<BackendCapabilities, BackendError>> {
        Box::pin(async {
            Ok(BackendCapabilities {
                tiled_panes: true,
                split_resize: true,
                tab_create: true,
                tab_close: true,
                tab_focus: true,
                tab_rename: true,
                session_scoped_tab_refs: true,
                session_scoped_pane_refs: true,
                pane_split: true,
                pane_close: true,
                pane_focus: true,
                pane_input_write: true,
                pane_paste_write: true,
                rendered_viewport_stream: true,
                rendered_viewport_snapshot: true,
                advisory_metadata_subscriptions: true,
                read_only_client_mode: true,
                ..BackendCapabilities::default()
            })
        })
    }

    fn discover_sessions(
        &self,
        _scope: BackendScope,
    ) -> BoxFuture<'_, Result<Vec<DiscoveredSession>, BackendError>> {
        Box::pin(async move {
            let output = match self.run(
                None,
                &[
                    "list-sessions",
                    "-F",
                    "#{session_name}\t#{session_windows}\t#{session_attached}",
                ],
            ) {
                Ok(output) => output,
                Err(error) if Self::is_no_server_running_error(&error) => return Ok(Vec::new()),
                Err(error) => return Err(error),
            };
            let mut sessions = Vec::new();
            for line in output.lines().filter(|line| !line.trim().is_empty()) {
                let mut fields = line.split('\t');
                let Some(session_name) = fields.next() else {
                    continue;
                };
                let target = TmuxTarget {
                    socket_name: self.socket_name.clone(),
                    session_name: session_name.to_string(),
                };
                sessions.push(DiscoveredSession {
                    route: target.route(),
                    title: Some(session_name.to_string()),
                });
            }

            Ok(sessions)
        })
    }

    fn create_session(
        &self,
        _spec: CreateSessionSpec,
    ) -> BoxFuture<'_, Result<BackendSessionBinding, BackendError>> {
        Box::pin(async {
            Err(BackendError::unsupported(
                "tmux sessions are imported, not created",
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
            if route.backend != BackendKind::Tmux {
                return Err(BackendError::invalid_input(
                    "tmux backend can only attach tmux routes",
                ));
            }
            let target = TmuxTarget::from_route(&route)?;
            backend.run(Some(&target), &["has-session", "-t", &target.session_name])?;

            Ok(Box::new(TmuxAttachedSession { backend: Arc::new(backend), session_id, target })
                as Box<dyn BackendSessionPort>)
        })
    }

    fn list_sessions(
        &self,
        _scope: BackendScope,
    ) -> BoxFuture<'_, Result<Vec<BackendSessionSummary>, BackendError>> {
        Box::pin(async {
            Err(BackendError::unsupported(
                "tmux backend does not expose canonical sessions directly",
                DegradedModeReason::ImportedForeignSession,
            ))
        })
    }
}
