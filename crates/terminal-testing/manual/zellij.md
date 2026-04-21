# Zellij Checklist

- Verify the local `zellij` version and note whether the surface is legacy or rich.
- Discover and import a live `Zellij` session through the daemon.
- For rich `0.44+`, verify topology, focused pane screen, subscriptions, and ordered mutation lane.
- Verify `send_input`, tab create, tab rename, tab focus, and tab close.
- Exercise viewport observation while switching tabs rapidly.
- Exercise detach/reattach around an imported `Zellij` session when the host environment supports it.
- Run `vim`, `less`, and `fzf` in a terminal pane and confirm render stability.
- Confirm legacy `0.43.x` imports fail with explicit `MissingCapability`.
