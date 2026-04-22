# Release Policy

**Checked**: 2026-04-22  
**Status**: frozen policy

## Goal

Define how the UI SDK is versioned, released, and hardened.

## Release Train

The SDK uses one coordinated release train at first.

Packages move together until:

- package ownership splits materially
- release cadence diverges materially
- compatibility management can handle independent trains

## Versioning Scheme

Use Semantic Versioning:

- `MAJOR` for breaking public API changes
- `MINOR` for backward-compatible feature growth
- `PATCH` for backward-compatible fixes

## Release Stages

### Local Workspace Stage

Before public publication:

- packages may exist only in the workspace
- contracts may still evolve
- no public npm compatibility promise is made

### Preview Stage

Preview releases use prerelease versions such as:

- `1.0.0-beta.1`
- `1.0.0-beta.2`

Use preview stage when:

- contracts are mostly frozen
- core exists
- elements exist
- demo migration is in progress or complete

### Stable Stage

Stable starts at:

- `1.0.0`

Stable requires:

- compatibility matrix published
- release checklist green
- packed-consumer smoke green
- deprecation policy active
- browser matrix reviewed

## Mandatory Release Gates

Every release candidate must satisfy:

- typecheck green
- unit tests green
- browser component tests green
- adapter conformance green
- packed package install smoke green
- demo consumer smoke green
- docs updated
- rollback path understood for the release cut

## Packed Package Rule

Reusable packages are not considered releasable unless they install and work from packed artifacts, not just source workspace linking.

## Deprecation Policy

When deprecating public API:

- document the deprecation
- provide migration guidance
- keep it alive for at least one `MINOR` release before removal

Removal requires the next `MAJOR`.

## Breaking Change Policy

Breaking changes require:

- ADR or explicit RFC-level review
- compatibility matrix update
- migration note
- release note callout

## Package Scope

No package may expose undocumented deep imports as public API.

`exports` defines the real contract.

## Tooling

Use:

- `@changesets/cli`
- packed tarball install tests
- compatibility matrix updates as release artifacts

## Rollback Rule

If a release candidate exposes a breaking regression in public package behavior, the release must be withdrawn or superseded immediately rather than silently tolerated.
