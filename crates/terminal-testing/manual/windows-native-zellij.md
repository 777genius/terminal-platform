# Windows Native + Zellij Checklist

- Verify Windows native PTY session create, attach, send input, and tab lifecycle.
- Verify staged and installed Node package flows on Windows, including the live `Zellij` import/control path through the package surface.
- Provision a real `Zellij 0.44+` binary and verify discover/import succeeds.
- Verify topology snapshot, screen snapshot, screen delta, and live viewport observation.
- Verify ordered mutation lane for `new_tab`, `rename_tab`, `focus_tab`, and `close_tab`.
- Run `vim`, `less`, and `fzf` in Windows native and imported `Zellij` panes when available, and confirm viewport fidelity.
- Stress resize churn while screen delta and live viewport observation remain active.
- Exercise Electron bridge lifecycle on Windows.
- Confirm `tmux` is absent from Windows acceptance and docs.
