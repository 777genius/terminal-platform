#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZellijProbe {
    pub(crate) version: String,
    pub(crate) surface: ZellijSurface,
}

impl ZellijProbe {
    pub(crate) fn parse(
        version_output: &str,
        root_help: Option<&str>,
        action_help: Option<&str>,
        dump_screen_help: Option<&str>,
        subscribe_help: Option<&str>,
    ) -> Self {
        let version = version_output.trim().to_string();
        let parsed = version.split_whitespace().find_map(parse_semver_triplet).unwrap_or((0, 0, 0));
        let surface =
            classify_surface(parsed, root_help, action_help, dump_screen_help, subscribe_help);

        Self { version, surface }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ZellijSurface {
    LegacyCli043,
    RichCli044Plus,
    Unknown,
}

fn classify_surface(
    parsed_version: (u64, u64, u64),
    root_help: Option<&str>,
    action_help: Option<&str>,
    dump_screen_help: Option<&str>,
    subscribe_help: Option<&str>,
) -> ZellijSurface {
    if let (Some(root_help), Some(action_help)) = (root_help, action_help) {
        let has_subscribe = help_contains_subcommand(root_help, "subscribe");
        let has_list_panes = help_contains_subcommand(action_help, "list-panes");
        let has_list_tabs = help_contains_subcommand(action_help, "list-tabs");
        let supports_dump_screen_ansi = help_supports_ansi(dump_screen_help, parsed_version);
        let supports_subscribe_ansi = help_supports_ansi(subscribe_help, parsed_version);
        if has_subscribe && has_list_panes && has_list_tabs {
            return if supports_dump_screen_ansi && supports_subscribe_ansi {
                ZellijSurface::RichCli044Plus
            } else {
                ZellijSurface::Unknown
            };
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

fn help_supports_ansi(help: Option<&str>, parsed_version: (u64, u64, u64)) -> bool {
    help.map(|help| help_contains_option(help, "--ansi")).unwrap_or(parsed_version >= (0, 44, 0))
}

fn help_contains_option(help: &str, option: &str) -> bool {
    help.lines().any(|line| line.split_whitespace().any(|token| token == option))
}

pub(crate) fn parse_semver_triplet(token: &str) -> Option<(u64, u64, u64)> {
    let stripped = token.trim().trim_start_matches('v');
    let mut parts = stripped.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;

    Some((major, minor, patch))
}
