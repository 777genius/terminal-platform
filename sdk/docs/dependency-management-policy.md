# Dependency Management Policy

**Checked**: 2026-04-22  
**Status**: frozen policy

## Goal

Define how the SDK workspace manages package manager choice, dependency versions, and lockfile discipline.

## Package Manager Choice

The SDK workspace uses:

- npm workspaces

This is chosen to align with the existing repository reality where `apps/terminal-demo` already uses `npm` and a committed `package-lock.json`.

The SDK must not introduce a second package manager by default.

## Lockfile Policy

The SDK workspace maintains its own lockfile at:

- `sdk/package-lock.json`

This lockfile is committed and treated as part of the reproducible workspace state.

## Node Baseline

The SDK workspace should align with the existing repo Node baseline unless a deliberate change is approved.

Current baseline to align with:

- `node >=20.19.0`

## Version Discipline

Dependency version policy:

- use exact versions for runtime-critical and toolchain-critical dependencies
- update shared toolchain dependencies intentionally, not ad hoc
- keep package versions boring and synchronized where practical

## Root Dependency Policy

Shared workspace tooling should live at the `sdk/` root when appropriate.

Examples:

- TypeScript
- Vitest
- build tooling
- release tooling

Package-local dependencies should exist only when they are genuinely package-specific.

## Internal Package Dependency Rule

Internal SDK packages depend on each other through documented workspace package names, not file paths and not demo paths.

## Anti-Pattern Rule

The following are not allowed by default:

- mixing npm and pnpm in the SDK workspace
- leaving lockfile generation implicit
- silently drifting Node baseline between packages
- using demo-owned dependencies as hidden SDK defaults
