# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/777genius/terminal-platform/releases/tag/terminal-daemon-client-v0.1.0) - 2026-04-20

### Added

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

- drain buffered subscription events on close
- harden corrupted saved session recovery
- close active subscriptions on daemon shutdown

### Other

- harden contract properties and pty waits
- harden pane surface stream coverage
