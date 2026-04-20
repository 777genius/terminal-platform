# Acceptance Evidence Capture

These checklists are the recorded acceptance evidence layer for ship-ready v1.

Required recorded passes before v1 acceptance:

- one Electron embed pass
- one Unix `tmux` pass
- one Windows `Zellij` pass

Use the platform/backend-specific checklists in this directory and turn every real failure
into an automated regression test where practical.

Recorded evidence can come from:

- a local hands-on acceptance run
- a hosted target-OS acceptance run, if the artifact notes include the workflow URL, executed commands, and real tool versions

Store recorded pass artifacts in [`runs/`](./runs/README.md).
