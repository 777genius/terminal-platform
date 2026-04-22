# ADR-012: Compatibility And Support Policy

**Status**: accepted  
**Date**: 2026-04-22

## Context

The SDK spans multiple packages and depends on runtime protocol compatibility. Release safety requires an explicit compatibility view.

## Decision

Maintain a compatibility matrix across:

- runtime protocol
- runtime types
- contracts
- core
- elements
- React wrappers

Also maintain package maturity labels:

- `internal`
- `beta`
- `stable`
- `deprecated`

## Consequences

- releases must document compatible combinations
- support promises become explicit
- deprecation and upgrade planning become tractable
