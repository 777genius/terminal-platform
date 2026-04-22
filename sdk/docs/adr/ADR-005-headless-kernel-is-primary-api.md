# ADR-005: Headless Kernel Is Primary API

**Status**: accepted  
**Date**: 2026-04-22

## Context

The SDK must support Lit, React, vanilla hosts, tests, and future UI surfaces without duplicating orchestration logic.

## Decision

Make `WorkspaceKernel` the primary public integration surface for the UI SDK.

The kernel owns orchestration, subscriptions, reducers, selectors, and lifecycle.

## Consequences

- components consume kernel instead of transport
- React wrappers consume kernel instead of duplicating logic
- tests can target core without DOM
- the project avoids component-owned transport lifecycle
