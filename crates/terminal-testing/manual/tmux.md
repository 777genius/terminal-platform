# tmux Checklist

- Run on macOS or Linux only.
- Start a real `tmux` server and discover the session through the daemon.
- Import a `tmux` session and verify topology plus screen snapshot.
- Verify `send_input`, tab create, tab rename, tab focus, and tab close.
- Verify pane subscription and topology subscription teardown.
- Exercise detach/reattach around an imported `tmux` session.
- Run `vim`, `less`, and `fzf` inside imported panes and confirm viewport fidelity.
- Confirm docs and UI do not imply Windows `tmux` support.
