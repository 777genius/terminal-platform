use crate::support::{assert_contains_all, assert_value, section_between};

pub(crate) fn verify_windows_conpty_vendor_patch(
    workspace_manifest: &str,
    vendored_psuedocon: &str,
) -> Result<(), String> {
    assert_contains_all(
        workspace_manifest,
        "workspace manifest Windows ConPTY patch",
        &["[patch.crates-io]", "portable-pty = { path = \"vendor/portable-pty\" }"],
    )?;
    assert_contains_all(
        vendored_psuedocon,
        "vendored portable-pty Windows ConPTY patch",
        &[
            "(CONPTY.CreatePseudoConsole)(",
            "Terminal Platform v1 intentionally",
            "flag 0 here",
            "plain UTF-8/VT I/O without",
        ],
    )?;

    let create_call =
        section_between(vendored_psuedocon, "(CONPTY.CreatePseudoConsole)(", "&mut con,")
            .ok_or_else(|| {
                "vendored portable-pty Windows ConPTY patch is missing CreatePseudoConsole call"
                    .to_string()
            })?;

    assert_value(
        create_call.contains("0,"),
        "vendored portable-pty Windows ConPTY patch must call CreatePseudoConsole with dwFlags = 0",
    )?;
    assert_value(
        !create_call.contains("PSUEDOCONSOLE_INHERIT_CURSOR"),
        "vendored portable-pty Windows ConPTY patch must not enable PSEUDOCONSOLE_INHERIT_CURSOR in CreatePseudoConsole",
    )?;
    assert_value(
        !create_call.contains("PSEUDOCONSOLE_RESIZE_QUIRK"),
        "vendored portable-pty Windows ConPTY patch must not enable PSEUDOCONSOLE_RESIZE_QUIRK in CreatePseudoConsole",
    )?;
    assert_value(
        !create_call.contains("PSEUDOCONSOLE_WIN32_INPUT_MODE"),
        "vendored portable-pty Windows ConPTY patch must not enable PSEUDOCONSOLE_WIN32_INPUT_MODE in CreatePseudoConsole",
    )?;

    Ok(())
}
