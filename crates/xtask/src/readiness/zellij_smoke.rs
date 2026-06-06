use crate::support::assert_contains_all;

pub(crate) fn verify_windows_zellij_package_smoke(
    node_smoke_test: &str,
    package_smoke_test: &str,
    package_install_smoke_test: &str,
) -> Result<(), String> {
    for (label, contents) in [
        ("node addon smoke", node_smoke_test),
        ("staged package smoke", package_smoke_test),
        ("installed package smoke", package_install_smoke_test),
    ] {
        assert_contains_all(
            contents,
            label,
            &[
                "#[cfg(windows)]",
                "windows_zellij_smoke_env",
                "TERMINAL_NODE_RUN_ZELLIJ_SMOKE",
                "TERMINAL_NODE_EXTERNAL_ZELLIJ_SESSION",
            ],
        )?;
    }

    Ok(())
}
