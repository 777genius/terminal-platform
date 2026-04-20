# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/777genius/terminal-platform/releases/tag/terminal-protocol-v0.1.0) - 2026-04-20

### Added

- harden daemon handshake contract
- gate saved session restore by compatibility
- add saved session prune api
- add saved session manifest metadata
- expose saved session restore semantics
- add saved session delete api
- restore native sessions from saved topology
- add saved session query api
- preserve degraded reasons in protocol errors
- expose backend capabilities over daemon protocol
- add tmux discover and import flow
- add explicit subscription close control
- add native subscription event streams
- add native screen delta transport
- add native mux command dispatch
- add native topology and screen read model
- add native session lifecycle skeleton
- add local socket daemon transport
- bootstrap workspace and core contracts

### Other

- harden contract properties and pty waits
