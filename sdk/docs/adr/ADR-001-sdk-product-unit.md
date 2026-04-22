# ADR-001: SDK Product Unit

**Status**: accepted  
**Date**: 2026-04-22

## Context

The repository already contains `apps/terminal-demo`, but the product goal is a reusable UI SDK that is independent from any single demo or app shell.

## Decision

Create the UI SDK as a separate product unit under `sdk/`.

`apps/terminal-demo` remains a consumer, showcase, and integration lab only.

## Consequences

- demo cannot own reusable truth
- SDK can evolve independently
- package boundaries become visible in the repo layout
- migration from demo becomes explicit instead of accidental
