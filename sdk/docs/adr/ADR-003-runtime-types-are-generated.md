# ADR-003: Runtime Types Are Generated

**Status**: accepted  
**Date**: 2026-04-22

## Context

The UI SDK needs stable TypeScript visibility into Rust truth without making Node or Electron packages the architectural center.

## Decision

Create `@terminal-platform/runtime-types` as a generated TypeScript mirror of Rust runtime truth.

This package contains generated DTO mirrors only.

## Consequences

- contracts can depend on one canonical TS mirror
- browser-facing SDK packages avoid coupling to Node leaf packages
- runtime mirror drift is reduced
- `runtime-types` must exclude UI opinions and transport policy
