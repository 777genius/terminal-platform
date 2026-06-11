# Deprecation Checkpoints

**Checked**: 2026-04-22  
**Status**: required release artifact

## Goal

Keep public SDK deprecations predictable for host applications.

Deprecation is part of the public package contract. It must not be hidden in implementation details, demo migration notes, or accidental TypeScript comments only.

## Deprecation Entry Template

Every public API deprecation must record:

- package name
- public entrypoint
- symbol or behavior
- first deprecated version
- earliest removal version
- migration guidance
- replacement API
- compatibility impact
- release note reference

## Required Checkpoints

Before marking an API deprecated:

- confirm the API is public through package `exports`
- confirm the replacement API exists and has tests
- confirm the migration path does not require demo internals
- add release notes for the deprecation
- keep the deprecated API alive for at least one `MINOR` release before removal

Before removing a deprecated API:

- require the next `MAJOR` release
- update the compatibility matrix if package compatibility changes
- update migration docs
- update packed-consumer smoke coverage if the removed API was covered there
- verify no SDK example imports the removed API

## Current Deprecations

None.
