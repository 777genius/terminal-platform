# Compatibility Matrix

**Checked**: 2026-04-22  
**Status**: required release artifact

## Goal

Track compatibility between the Rust runtime truth and the UI SDK packages.

## Version Axes

- Rust runtime protocol
- `@terminal-platform/runtime-types`
- `@terminal-platform/workspace-contracts`
- `@terminal-platform/workspace-core`
- `@terminal-platform/workspace-elements`
- `@terminal-platform/workspace-react`

## Current Matrix

| Runtime Protocol | Runtime Types | Workspace Contracts | Workspace Core | Workspace Elements | Workspace React | Status | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 0.2 | 0.1.0 | 0.1.0 | 0.1.0 | 0.1.0 | 0.1.0 | preview | Initial v1 preview row validated by `npm run check:compatibility-matrix`; stable status still requires the remaining release gates. |

## Verification

Run `npm run check:compatibility-matrix` from `sdk/` before release cuts and after changes to runtime protocol constants or public SDK package versions.

## Rules

- stable releases must update this matrix
- breaking contract changes require matrix updates
- preview releases may add provisional rows
- unsupported combinations must be called out explicitly

## Interpretation

Statuses should use:

- `planned`
- `preview`
- `stable`
- `deprecated`
- `unsupported`

## Responsibility

The release owner must update this file whenever any of the following changes:

- runtime protocol shape
- generated runtime types contract
- public SDK contracts
- element public API
- React wrapper public API
