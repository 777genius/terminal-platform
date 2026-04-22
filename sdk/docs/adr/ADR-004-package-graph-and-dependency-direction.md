# ADR-004: Package Graph And Dependency Direction

**Status**: accepted  
**Date**: 2026-04-22

## Context

Without a hard dependency graph, reusable SDK code drifts toward framework-first and demo-first coupling.

## Decision

Freeze the following dependency direction:

```text
runtime-types -> workspace-contracts -> workspace-core
foundation -> workspace-core
workspace-contracts + workspace-core -> adapters
design-tokens -> workspace-elements
workspace-core -> workspace-elements
workspace-core + workspace-elements -> workspace-react
apps/terminal-demo -> public sdk packages only
```

## Consequences

- core remains framework-free
- elements remain consumer leaves
- adapters remain external integration leaves
- demo cannot leak inward
