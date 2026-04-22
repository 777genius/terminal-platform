# Diagnostics Model

**Checked**: 2026-04-22  
**Status**: frozen policy

## Goal

Define how the UI SDK exposes diagnostics, telemetry seams, and observable failure states.

## Principle

Diagnostics must be explicit and queryable, not hidden in random console output or transport-specific side channels.

## Public Diagnostics Surface

The kernel exposes diagnostics through structured interfaces and read models.

Diagnostics are intended to cover at least:

- connection health
- adapter state
- degraded capability state
- recoverable error state

## Telemetry Sink

Low-level telemetry integration should go through explicit sink interfaces in `foundation`, not hidden imports or global singletons.

## UI Rule

The UI may surface diagnostics and state hints, but must not rely on ad hoc log scraping.

## Adapter Rule

Adapters are responsible for converting transport/runtime failures into explicit diagnostics and contract-compatible errors.

## Testing Rule

Diagnostics paths should be exercised in tests for:

- reconnect behavior
- stale callback handling
- degraded capability scenarios
- close and dispose behavior
