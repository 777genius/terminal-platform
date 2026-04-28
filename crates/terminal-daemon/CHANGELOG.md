# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/777genius/terminal-platform/releases/tag/terminal-daemon-v0.1.0) - 2026-04-28

### Added

- *(runtime)* expose session health through node api
- *(workspace)* add session health and simplify demo ux
- *(sdk)* add workspace sdk and demo consumer
- harden daemon handshake contract
- gate saved session restore by compatibility
- add saved session prune api
- add saved session manifest metadata
- expose saved session restore semantics
- add saved session delete api
- restore native sessions from saved topology
- add saved session query api
- add native session save persistence
- add native layout override dispatch
- add native pane lifecycle controls
- add tmux pane close and resize controls
- add tmux pane lifecycle controls
- add tmux tab lifecycle controls
- probe zellij control surface from cli help
- preserve degraded reasons in protocol errors
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

- harden hosted ci readiness
- close active subscriptions on daemon shutdown

### Other

- merge origin main before push
- *(transport)* extract local socket transport crate
- *(runtime)* extract terminal runtime crate
- *(application)* split daemon and session services by use case
- *(daemon)* extract runtime ports and protocol adapters
