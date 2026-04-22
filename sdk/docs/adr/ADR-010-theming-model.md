# ADR-010: Theming Model

**Status**: accepted  
**Date**: 2026-04-22

## Context

The SDK needs a durable theming system that works with Shadow DOM and multiple host environments.

## Decision

Use:

- DTCG-compatible token source
- CSS custom properties as runtime theme contract
- `::part()` for structural styling
- slots for content extension

Internal DOM structure is not public theming API.

## Consequences

- theming scales better across hosts
- Shadow DOM remains encapsulated
- token authoring and runtime application remain separate concerns
