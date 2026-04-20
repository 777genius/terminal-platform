# Manual QA Capture

These checklists are the human acceptance layer for ship-ready v1.

Required recorded passes before v1 acceptance:

- one Electron embed pass
- one Unix `tmux` pass
- one Windows `Zellij` pass

Use the platform/backend-specific checklists in this directory and turn every real failure
into an automated regression test where practical.

Store recorded pass artifacts in [`runs/`](./runs/README.md).
