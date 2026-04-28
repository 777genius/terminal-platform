# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/777genius/terminal-platform/releases/tag/terminal-daemon-client-v0.1.0) - 2026-04-28

### Added

- *(runtime)* expose session health through node api
- *(workspace)* add session health and simplify demo ux
- add ship-ready v1 verification closeout
- harden daemon handshake contract
- gate saved session restore by compatibility
- add saved session prune api
- add saved session manifest metadata
- expose saved session restore semantics
- add saved session delete api
- restore native sessions from saved topology
- add saved session query api
- refresh session summaries after mux title changes
- add native session save persistence
- add native layout override dispatch
- add native pane lifecycle controls
- add tmux pane close and resize controls
- add tmux pane lifecycle controls
- add tmux tab lifecycle controls
- add tmux close-tab control path
- refine control capability negotiation
- expose backend capabilities over daemon protocol
- add tmux discover and import flow
- add explicit subscription close control
- add native subscription event streams
- add partial native screen deltas
- add native screen delta transport
- add native pty session runtime
- harden mux command protocol errors
- add native mux command dispatch
- add native topology and screen read model
- add native session lifecycle skeleton
- add local socket daemon transport
- add daemon request routing smoke flow
- bootstrap workspace and core contracts

### Fixed

- submit windows smoke input with crlf
- echo windows smoke input reliably
- resolve windows shell and zellij probes
- harden windows pty and zellij smoke
- harden windows native and zellij smoke
- bound windows pty and zellij smoke waits
- harden windows live smoke fixtures
- stabilize windows smoke input and zellij mutations
- harden windows zellij readiness and echo probes
- stabilize windows echo smoke and zellij import fallback
- harden windows ci host smoke flows
- stabilize zellij smoke flows and shutdown recovery
- harden ci nextest hangs
- harden hosted ci readiness
- drain buffered subscription events on close
- harden corrupted saved session recovery
- close active subscriptions on daemon shutdown

### Other

- merge origin main before push
- *(transport)* extract local socket transport crate
- *(runtime)* extract terminal runtime crate
- serialize windows live runtime smoke
- harden contract properties and pty waits
- harden pane surface stream coverage
