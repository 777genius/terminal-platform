use crate::{ZellijProbe, ZellijSurface, capabilities_for_surface, parse_semver_triplet};

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
fn rich_surface_advertises_full_rendered_history_capture() {
    let capabilities = capabilities_for_surface(ZellijSurface::RichCli044Plus);

    assert!(capabilities.rendered_viewport_snapshot);
    assert!(capabilities.rendered_viewport_stream);
    assert!(capabilities.rendered_scrollback_snapshot);
}

#[test]
fn legacy_surface_does_not_overpromise_rendered_history_capture() {
    let capabilities = capabilities_for_surface(ZellijSurface::LegacyCli043);

    assert!(!capabilities.rendered_viewport_snapshot);
    assert!(!capabilities.rendered_viewport_stream);
    assert!(!capabilities.rendered_scrollback_snapshot);
    assert!(capabilities.read_only_client_mode);
}
