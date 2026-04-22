# ADR-009: Event Model

**Status**: accepted  
**Date**: 2026-04-22

## Context

Terminal UI systems often devolve into one generic event bus that mixes commands, transport frames, and render deltas.

## Decision

Adopt a three-plane model:

- control plane via kernel commands
- observation plane via core state and selectors
- screen plane via dedicated renderer inputs

Public DOM events are reserved for semantic UI outputs only.

## Consequences

- command handling stays typed and explicit
- high-volume screen traffic does not pollute app events
- component public events remain small and stable
