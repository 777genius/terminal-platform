use crate::support::assert_contains_all;

pub(crate) fn verify_v1_release_configs(
    release_plz_config: &str,
    deny_config: &str,
) -> Result<(), String> {
    assert_contains_all(
        release_plz_config,
        "release-plz config",
        &[
            "[workspace]",
            "allow_dirty = false",
            "git_release_enable = false",
            "pr_branch_prefix = \"release-plz-\"",
            "semver_check = false",
        ],
    )?;
    assert_contains_all(
        deny_config,
        "cargo-deny config",
        &[
            "[advisories]",
            "yanked = \"deny\"",
            "[licenses]",
            "[sources]",
            "unknown-registry = \"deny\"",
            "unknown-git = \"deny\"",
        ],
    )?;

    Ok(())
}
