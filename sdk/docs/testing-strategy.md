# Testing Strategy

**Checked**: 2026-04-22  
**Status**: frozen test strategy

## Goal

Define the minimum test architecture required for the UI SDK to be trustworthy and releasable.

## Test Layers

### Foundation

Test type:

- unit tests

Coverage focus:

- store behavior
- lifecycle ownership
- async lanes
- stale guard correctness

### Runtime Types

Test type:

- generation smoke tests
- schema drift detection

Coverage focus:

- generated output stability
- mapping correctness

### Workspace Contracts

Test type:

- type-level and structural tests

Coverage focus:

- IDs
- error shapes
- command payloads
- observation payloads

### Workspace Core

Test type:

- unit tests
- reducer tests
- selector tests
- lifecycle tests

Coverage focus:

- command handling
- read model updates
- diagnostics behavior
- dispose semantics

### Adapters

Test type:

- contract tests
- reconnect tests
- race tests
- close semantics tests

Coverage focus:

- normalization
- subscription ownership
- reconnect behavior
- stale callback safety

### Elements

Test type:

- browser tests
- interaction tests
- accessibility smoke tests

Coverage focus:

- props, methods, and events
- shadow DOM behavior
- slots and parts
- keyboard behavior
- overlay behavior

Browser rule:

- elements are tested in a real browser environment, not fake DOM only
- browser coverage must include Chromium first, with Firefox and WebKit in release-grade validation where supported

### React Layer

Test type:

- wrapper tests
- hook tests

Coverage focus:

- prop mapping
- event mapping
- no business logic duplication

### Demo

Test type:

- consumer integration smoke

Coverage focus:

- imports only public SDK packages
- host composition still works

## Packed Package Rule

Every stable-ready package must be validated from packed artifacts, not only from workspace linking.

## Required Release Gates

Before stable release:

- typecheck green
- unit tests green
- browser tests green
- adapter conformance green
- packed-consumer smoke green
- demo consumer smoke green
- release browser matrix green

## Performance Testing

At minimum:

- renderer smoke under realistic screen loads
- subscription fan-out sanity checks
- overlay path sanity checks

Detailed perf budgets are tracked in release planning, not hidden in ad hoc notes.
