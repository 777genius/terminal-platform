# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/777genius/terminal-platform/releases/tag/terminal-node-v0.1.0) - 2026-04-28

### Added

- *(runtime)* expose session health through node api
- *(workspace)* add session health and simplify demo ux
- *(sdk)* add workspace sdk and demo consumer
- add ship-ready v1 verification closeout
- add ordered zellij mutation lane
- add rich zellij import surface
- add node subscription bridge
- expand node host control plane
- add terminal node host facade
- bootstrap workspace and core contracts

### Fixed

- submit windows smoke input with crlf
- echo windows smoke input reliably
- harden windows pty and zellij smoke
- cap windows zellij discovery waits
- harden windows native and zellij smoke
- bound windows pty and zellij smoke waits
- harden windows live smoke fixtures
- stabilize windows smoke input and zellij mutations
- stabilize windows echo smoke and zellij import fallback
- harden windows interactive smoke readiness
- harden windows ci host smoke flows
- stabilize zellij smoke flows and shutdown recovery
- harden zellij ci closeout
- harden ci nextest hangs
- harden hosted ci readiness
- drain node subscriptions under backpressure
- close active subscriptions on daemon shutdown

### Other

- merge origin main before push
- *(runtime)* extract terminal runtime crate
- tighten windows smoke failure budget
- serialize windows live runtime smoke
- stress node host restart and subscription cycles
- add terminal-node restart recovery smoke
