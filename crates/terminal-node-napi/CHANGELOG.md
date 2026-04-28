# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/777genius/terminal-platform/releases/tag/terminal-node-napi-v0.1.0) - 2026-04-28

### Added

- *(terminal-demo)* harden production workspace demo
- *(runtime)* expose session health through node api
- *(runtime)* split terminal demo into runtime spine features
- add v1 readiness audit and closeout docs
- add ship-ready v1 verification closeout
- add ordered zellij mutation lane
- add rich zellij import surface
- add electron preload bridge helpers
- add electron bridge helpers
- add node session state helpers
- add node session watch helper
- add node watch helpers
- add node subscription bridge
- expand node host control plane
- add target-aware node addon manifest
- verify and pack node sdk artifacts
- add local node package build flow
- add staged node sdk package
- add napi leaf adapter for terminal node

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
- await electron bridge stop drain
- keep watch session state focused screen consistent
- harden package daemon restart recovery
- drain node subscriptions under backpressure
- close active subscriptions on daemon shutdown

### Other

- *(terminal-node-napi)* clean package temp paths
- merge origin main before push
- tighten windows smoke failure budget
- bound node smoke processes
- serialize windows live runtime smoke
- polish public repository docs and policies
- harden addon and c api lifecycle smoke
- harden direct node addon lifecycle smoke
- harden installed package recovery flows
- add repeated electron watch lifecycle smoke
- add timeout guards to node smoke helpers
- harden electron bridge lifecycle smoke
- add node package tarball install smoke
- harden node package manifest resolution
