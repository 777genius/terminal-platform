# ADR-011: Publishing And SemVer

**Status**: accepted  
**Date**: 2026-04-22

## Context

Large SDKs decay when their public package surfaces are implicit or when undocumented subpaths leak into usage.

## Decision

Publish packages as ESM-first with strict `exports` and bundled `.d.ts`.

SemVer applies to all stable public package surfaces.

## Consequences

- public API surface is explicit
- deep imports are not part of the contract
- breaking changes are easier to reason about
- stable releases require release and deprecation discipline
