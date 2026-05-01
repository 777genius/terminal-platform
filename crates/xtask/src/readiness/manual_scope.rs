use crate::support::assert_contains_all;

pub(crate) fn verify_manual_qa_scope(
    electron_checklist: &str,
    native_checklist: &str,
    tmux_checklist: &str,
    zellij_checklist: &str,
    windows_native_zellij_checklist: &str,
) -> Result<(), String> {
    assert_contains_all(
        electron_checklist,
        "Electron manual checklist",
        &["main-process bridge", "preload API", "resize churn"],
    )?;
    assert_contains_all(
        native_checklist,
        "native manual checklist",
        &["`vim`", "`less`", "`fzf`", "resize churn"],
    )?;
    assert_contains_all(
        tmux_checklist,
        "tmux manual checklist",
        &["Import a `tmux` session", "detach/reattach", "`vim`", "`less`", "`fzf`"],
    )?;
    assert_contains_all(
        zellij_checklist,
        "Zellij manual checklist",
        &[
            "import a live `Zellij` session",
            "ordered mutation lane",
            "viewport observation",
            "detach/reattach",
            "`vim`",
            "`less`",
            "`fzf`",
        ],
    )?;
    assert_contains_all(
        windows_native_zellij_checklist,
        "Windows Native + Zellij manual checklist",
        &[
            "live `Zellij` import/control path through the package surface",
            "topology snapshot",
            "screen snapshot",
            "screen delta",
            "live viewport observation",
            "`new_tab`",
            "`rename_tab`",
            "`focus_tab`",
            "`close_tab`",
            "`vim`",
            "`less`",
            "`fzf`",
            "resize churn",
            "Electron bridge lifecycle",
            "`tmux` is absent",
        ],
    )?;

    Ok(())
}
