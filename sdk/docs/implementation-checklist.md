# Implementation Checklist

**Checked**: 2026-04-22  
**Status**: execution checklist

Use this file as the operational progress list while implementing the SDK plan.

## Phase 0 - ADR Freeze

- [x] Create `sdk/docs/adr`
- [x] Write ADR-001 through ADR-012
- [x] Write event model
- [x] Write theming model
- [x] Write accessibility model
- [x] Write security model
- [x] Write degraded semantics policy
- [x] Write diagnostics model
- [x] Write performance model
- [x] Write product expansion model
- [x] Write build and CI model
- [x] Write runtime types generation model
- [x] Write examples policy
- [x] Write dependency management policy
- [x] Write workspace bootstrap spec
- [x] Write package template spec
- [x] Write release policy
- [x] Write support policy
- [x] Write compatibility matrix template
- [x] Freeze package graph

## Phase 1 - SDK Bootstrap

- [x] Create `sdk/.gitignore`
- [x] Create `sdk/.changeset/config.json`
- [x] Create `sdk/package.json`
- [x] Create `sdk/package-lock.json`
- [x] Create `sdk/tsconfig.base.json`
- [x] Create `sdk/vitest.config.ts`
- [x] Create package skeletons
- [x] Freeze package names and public entrypoints
- [x] Freeze package template expectations
- [x] Freeze package manager and Node baseline
- [x] Add build scripts
- [x] Add test scripts
- [x] Add changesets setup
- [x] Add strict `exports`
- [x] Verify all empty packages build

## Phase 2 - Runtime Types

- [x] Create generation script
- [x] Generate TS mirror from Rust truth
- [x] Add smoke tests
- [x] Add codegen drift check
- [x] Add compat version tagging
- [x] Verify no Node/Electron coupling

## Phase 3 - Workspace Contracts

- [x] Define IDs
- [x] Define capability model
- [x] Define ports
- [x] Define commands
- [x] Define observations
- [x] Define public errors
- [x] Define compatibility metadata
- [x] Verify no backend-native refs leak

## Phase 4 - Foundation

- [x] Add `ExternalStore`
- [x] Add `ResourceScope`
- [x] Add `Disposable`
- [x] Add `AsyncLane`
- [x] Add `GenerationToken`
- [x] Add base errors
- [x] Add telemetry sink
- [x] Verify foundation is minimal

## Phase 5 - Workspace Core

- [x] Add `WorkspaceKernel`
- [x] Add core services
- [ ] Add reducers
- [x] Add selectors
- [x] Add read models
- [x] Add lifecycle ownership
- [x] Add stale-result guards
- [x] Verify core works without UI

## Phase 6 - Adapters

- [x] Add websocket adapter
- [x] Add preload adapter
- [x] Add memory adapter
- [x] Add codec and normalization
- [x] Add reconnect policy
- [ ] Add explicit diagnostics mapping
- [x] Add contract tests
- [x] Add race tests

## Phase 7 - Design Tokens

- [x] Add token source
- [ ] Add transforms
- [ ] Add CSS vars generation
- [x] Add theme manifests
- [ ] Document token taxonomy
- [x] Freeze custom element tag namespace

## Phase 8 - Elements v1

- [x] Add public composite elements
- [x] Add Shadow DOM styling
- [ ] Add slots and parts
- [x] Add documented element registration helper
- [ ] Add accessibility baseline
- [ ] Add documented keyboard and focus behavior
- [ ] Add renderer seam
- [ ] Add overlays
- [x] Verify kernel-only integration

## Phase 9 - Demo Migration

- [ ] Replace demo deep imports
- [x] Import only public SDK packages
- [ ] Remove reusable truth from demo ownership
- [x] Verify demo can be treated as pure consumer

## Phase 10 - React Layer

- [x] Add wrappers
- [x] Add hooks
- [x] Add JSX typings
- [ ] Add typed event mapping
- [x] Verify no duplicate business logic

## Phase 11 - Hardening

- [ ] Add packed package smoke tests
- [ ] Run browser test matrix
- [ ] Fill compatibility matrix
- [ ] Add perf budgets
- [ ] Confirm rollback plan for release cut
- [ ] Add deprecation policy checkpoints
- [ ] Add release checklist
- [ ] Add integration samples
- [ ] Validate samples use only public entrypoints
- [ ] Verify production-grade release gates
