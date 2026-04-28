# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/777genius/terminal-platform/releases/tag/terminal-backend-native-v0.1.0) - 2026-04-28

### Added

- *(runtime)* expose session health through node api
- add ship-ready v1 verification closeout
- restore native sessions from saved topology
- add native session save persistence
- add native layout override dispatch
- add native layout-aware pane resize
- add native pane geometry reflow
- add native pane lifecycle controls
- add tmux pane close and resize controls
- add tmux pane lifecycle controls
- add tmux tab lifecycle controls
- add tmux close-tab control path
- refine control capability negotiation
- add tmux discover and import flow
- add explicit subscription close control
- add native subscription event streams
- add partial native screen deltas
- add native screen delta transport
- add emulator-backed native screen snapshots
- add native pty session runtime
- add native mux command dispatch
- add native topology and screen read model
- add native session lifecycle skeleton
- bootstrap workspace and core contracts

### Fixed

- echo windows smoke input reliably
- resolve windows shell and zellij probes
- harden hosted ci readiness

### Other

- merge origin main before push
- *(native)* split backend into engine application and subscriptions
- *(runtime)* extract terminal runtime crate
- harden contract properties and pty waits
