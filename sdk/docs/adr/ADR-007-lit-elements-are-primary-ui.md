# ADR-007: Lit Elements Are Primary UI

**Status**: accepted  
**Date**: 2026-04-22

## Context

The SDK needs a portable UI layer that works across host stacks and modern browser-like environments.

## Decision

Use Lit-based Web Components as the primary portable UI layer.

Public component entrypoints live in `@terminal-platform/workspace-elements`.

## Consequences

- UI is portable beyond React
- Shadow DOM, slots, and parts become first-class tools
- React becomes optional, not foundational
- component API must be designed as web component API, not React API
