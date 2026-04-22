# ADR-006: Adapters Are Anti-Corruption Layer

**Status**: accepted  
**Date**: 2026-04-22

## Context

Transport/runtime integration will vary across WebSocket, Electron preload, and future memory or remote transports.

## Decision

All transport/runtime integration lives in dedicated adapter packages.

Adapters own:

- codec
- normalization
- retry and reconnect
- subscription ownership at the transport edge

## Consequences

- core stays transport-agnostic
- raw DTOs stay adapter-private
- backend/runtime peculiarities are normalized before reaching contracts/core
