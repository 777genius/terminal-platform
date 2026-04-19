use std::process::Command;

use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendScope, BackendSessionBinding, BackendSessionPort,
    BackendSessionSummary, BoxFuture, CreateSessionSpec, DiscoveredSession, MuxBackendPort,
};
use terminal_domain::{
    BackendKind, DegradedModeReason, ExternalSessionRef, RouteAuthority, SessionRoute,
};

const ZELLIJ_ROUTE_NAMESPACE: &str = "zellij_session";

#[derive(Debug, Default)]
pub struct ZellijBackend;

impl ZellijBackend {
    #[must_use]
    pub fn kind(&self) -> BackendKind {
        BackendKind::Zellij
    }

    fn run(&self, args: &[&str]) -> Result<String, BackendError> {
        let output = Command::new("zellij").args(args).output().map_err(|error| {
            BackendError::transport(format!("zellij command failed to spawn: {error}"))
        })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BackendError::transport(format!(
                "zellij command failed: {}",
                stderr.trim()
            )));
        }

        String::from_utf8(output.stdout)
            .map_err(|error| BackendError::internal(format!("zellij output is not utf8: {error}")))
    }

    fn probe(&self) -> Result<ZellijProbe, BackendError> {
        let version_output = self.run(&["--version"])?;
        let root_help = self.run(&["--help"]).ok();
        let action_help = self.run(&["action", "--help"]).ok();

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
                    read_only_client_mode: true,
                    tab_create: true,
                    tab_close: true,
                    tab_focus: true,
                    tab_rename: true,
                    pane_input_write: true,
                    pane_paste_write: true,
                    rendered_viewport_stream: true,
                    rendered_viewport_snapshot: true,
                    advisory_metadata_subscriptions: true,
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
            let output = self.run(&["list-sessions", "--short", "--no-formatting"])?;
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
        route: SessionRoute,
    ) -> BoxFuture<'_, Result<Box<dyn BackendSessionPort>, BackendError>> {
        Box::pin(async move {
            let target = ZellijTarget::from_route(&route)?;
            let probe = self.probe()?;
            let sessions = self.discover_sessions(BackendScope::CurrentUser).await?;
            if !sessions.iter().any(|session| session.route == route) {
                return Err(BackendError::not_found(format!(
                    "zellij session '{}' is not active",
                    target.session_name
                )));
            }

            let (message, degraded_reason) = match probe.surface {
                ZellijSurface::RichCli044Plus => (
                    "zellij rich import surface is not implemented yet".to_string(),
                    DegradedModeReason::NotYetImplemented,
                ),
                ZellijSurface::LegacyCli043 => (
                    format!(
                        "zellij {} does not expose the list-panes/list-tabs/subscribe surface required for imported attach",
                        probe.version
                    ),
                    DegradedModeReason::MissingCapability,
                ),
                ZellijSurface::Unknown => (
                    format!(
                        "zellij {} could not be matched to a supported control surface",
                        probe.version
                    ),
                    DegradedModeReason::MissingCapability,
                ),
            };

            Err(BackendError::unsupported(message, degraded_reason))
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZellijTarget {
    session_name: String,
}

impl ZellijTarget {
    fn from_route(route: &SessionRoute) -> Result<Self, BackendError> {
        if route.authority != RouteAuthority::ImportedForeign {
            return Err(BackendError::invalid_input("zellij route must be imported_foreign"));
        }
        let external = route.external.as_ref().ok_or_else(|| {
            BackendError::invalid_input("zellij route is missing external reference")
        })?;
        if external.namespace != ZELLIJ_ROUTE_NAMESPACE {
            return Err(BackendError::invalid_input("zellij route namespace is invalid"));
        }
        let session_name = external
            .value
            .strip_prefix("session=")
            .ok_or_else(|| BackendError::invalid_input("zellij route is missing session"))?;

        Ok(Self { session_name: session_name.to_string() })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZellijProbe {
    version: String,
    surface: ZellijSurface,
}

impl ZellijProbe {
    fn parse(version_output: &str, root_help: Option<&str>, action_help: Option<&str>) -> Self {
        let version = version_output.trim().to_string();
        let parsed = version.split_whitespace().find_map(parse_semver_triplet).unwrap_or((0, 0, 0));
        let surface = classify_surface(parsed, root_help, action_help);

        Self { version, surface }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ZellijSurface {
    LegacyCli043,
    RichCli044Plus,
    Unknown,
}

fn classify_surface(
    parsed_version: (u64, u64, u64),
    root_help: Option<&str>,
    action_help: Option<&str>,
) -> ZellijSurface {
    if let (Some(root_help), Some(action_help)) = (root_help, action_help) {
        let has_subscribe = help_contains_subcommand(root_help, "subscribe");
        let has_list_panes = help_contains_subcommand(action_help, "list-panes");
        let has_list_tabs = help_contains_subcommand(action_help, "list-tabs");
        if has_subscribe && has_list_panes && has_list_tabs {
            return ZellijSurface::RichCli044Plus;
        }

        let has_query_tab_names = help_contains_subcommand(action_help, "query-tab-names");
        let has_dump_layout = help_contains_subcommand(action_help, "dump-layout");
        if has_query_tab_names || has_dump_layout {
            return ZellijSurface::LegacyCli043;
        }
    }

    if parsed_version >= (0, 44, 0) {
        ZellijSurface::RichCli044Plus
    } else if parsed_version >= (0, 43, 0) {
        ZellijSurface::LegacyCli043
    } else {
        ZellijSurface::Unknown
    }
}

fn help_contains_subcommand(help: &str, subcommand: &str) -> bool {
    help.lines().map(str::trim_start).any(|line| line.starts_with(subcommand))
}

fn parse_semver_triplet(token: &str) -> Option<(u64, u64, u64)> {
    let stripped = token.trim().trim_start_matches('v');
    let mut parts = stripped.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;

    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use terminal_domain::{RouteAuthority, SessionRoute};

    use super::{
        ZELLIJ_ROUTE_NAMESPACE, ZellijProbe, ZellijSurface, ZellijTarget, parse_semver_triplet,
    };

    #[test]
    fn parses_legacy_surface_from_cli_help() {
        let probe = ZellijProbe::parse(
            "zellij 0.43.1",
            Some("SUBCOMMANDS:\n    action\n    attach\n"),
            Some("SUBCOMMANDS:\n    dump-layout\n    query-tab-names\n"),
        );

        assert_eq!(probe.surface, ZellijSurface::LegacyCli043);
    }

    #[test]
    fn parses_rich_surface_from_cli_help() {
        let probe = ZellijProbe::parse(
            "zellij 0.44.1",
            Some("SUBCOMMANDS:\n    action\n    subscribe\n"),
            Some("SUBCOMMANDS:\n    list-panes\n    list-tabs\n"),
        );

        assert_eq!(probe.surface, ZellijSurface::RichCli044Plus);
    }

    #[test]
    fn falls_back_to_version_when_help_is_missing() {
        let probe = ZellijProbe::parse("zellij 0.43.1", None, None);

        assert_eq!(probe.surface, ZellijSurface::LegacyCli043);
    }

    #[test]
    fn parses_semver_triplet() {
        assert_eq!(parse_semver_triplet("0.43.1"), Some((0, 43, 1)));
        assert_eq!(parse_semver_triplet("v0.44.0"), Some((0, 44, 0)));
    }

    #[test]
    fn roundtrips_zellij_route_target() {
        let route = SessionRoute {
            backend: terminal_domain::BackendKind::Zellij,
            authority: RouteAuthority::ImportedForeign,
            external: Some(terminal_domain::ExternalSessionRef {
                namespace: ZELLIJ_ROUTE_NAMESPACE.to_string(),
                value: "session=workspace".to_string(),
            }),
        };

        let target = ZellijTarget::from_route(&route).expect("route should decode");
        assert_eq!(target.session_name, "workspace");
    }

    #[test]
    fn rejects_invalid_zellij_route_namespace() {
        let route = SessionRoute {
            backend: terminal_domain::BackendKind::Zellij,
            authority: RouteAuthority::ImportedForeign,
            external: Some(terminal_domain::ExternalSessionRef {
                namespace: "other".to_string(),
                value: "session=workspace".to_string(),
            }),
        };

        let error = ZellijTarget::from_route(&route).expect_err("route should fail");
        assert_eq!(error.kind, terminal_backend_api::BackendErrorKind::InvalidInput);
    }
}
