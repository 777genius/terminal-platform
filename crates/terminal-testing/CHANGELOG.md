# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/777genius/terminal-platform/releases/tag/terminal-testing-v0.1.0) - 2026-04-28

### Added

- *(runtime)* expose session health through node api
- *(runtime)* split terminal demo into runtime spine features
- add v1 readiness audit and closeout docs
- add ship-ready v1 verification closeout
- add ordered zellij mutation lane
- add rich zellij import surface
- expand node host control plane
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
- add native layout-aware pane resize
- add native pane geometry reflow
- add native pane lifecycle controls
- add tmux pane close and resize controls
- add tmux pane lifecycle controls
- add tmux tab lifecycle controls
- preserve degraded reasons in protocol errors
- add tmux close-tab control path
- refine control capability negotiation
- add conservative tmux control commands
- expose backend capabilities over daemon protocol
- add zellij discovery compatibility gate
- add tmux subscription observe lane
- add tmux discover and import flow
- add explicit subscription close control
- add native subscription event streams
- add partial native screen deltas
- add native screen delta transport
- add native pty session runtime
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
- cap windows zellij discovery waits
- harden windows native and zellij smoke
- bound windows pty and zellij smoke waits
- harden windows live smoke fixtures
- stabilize windows smoke input and zellij mutations
- tolerate zellij cli action syntax variants
- harden windows zellij readiness and echo probes
- stabilize windows echo smoke and zellij import fallback
- harden windows interactive smoke readiness
- harden windows ci host smoke flows
- stabilize zellij smoke flows and shutdown recovery
- harden zellij ci closeout
- harden ci nextest hangs
- spawn zellij test sessions in background
- harden hosted ci readiness
- drain buffered subscription events on close

### Other

- merge origin main before push
- *(runtime)* extract terminal runtime crate
- tighten windows smoke failure budget
- serialize windows live runtime smoke
- record unix and electron v1 evidence
- harden tmux fullscreen acceptance
- harden manual v1 closeout flow
- add release closeout scaffolding and repo automation
- harden readiness audit and ci actions
- harden contract properties and pty waits
- harden pane surface stream coverage
