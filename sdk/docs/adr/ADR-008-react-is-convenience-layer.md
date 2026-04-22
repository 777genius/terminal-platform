# ADR-008: React Is Convenience Layer

**Status**: accepted  
**Date**: 2026-04-22

## Context

React is an important host ecosystem, but the product goal is broader than React-only adoption.

## Decision

Create `@terminal-platform/workspace-react` only as a thin convenience layer over core and elements.

React wrappers must not become the architectural center or hold product truth.

## Consequences

- React users get good DX
- React package stays thin
- no duplicate business logic
- the SDK remains portable for non-React consumers
