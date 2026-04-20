# Electron Embed Checklist

- Create the main-process bridge and preload API against a live daemon.
- Verify renderer session attach, topology watch, pane watch, and `watchSessionState`.
- Confirm dispose semantics for main bridge and preload API drain active watchers cleanly.
- Stop the daemon under an active watcher and verify closure is explicit, not hanging.
- Restart the daemon on the same address and confirm client recovery behavior.
- Verify focused screen stays aligned with topology during pane/tab changes.
