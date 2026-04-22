# Migration Guide

**Checked**: 2026-04-22  
**Status**: frozen migration policy

## Goal

Move from the current demo feature implementation to the independent UI SDK without letting demo become the source of truth.

## Principle

This is not a code lift from demo into `sdk/`.

This is a controlled migration where:

- `sdk/` is created first as a new product unit
- demo becomes a downstream consumer
- existing demo code is used only as donor seam and implementation reference

## Source Mapping

Current demo feature architecture can donate concepts:

- current `contracts/` -> future `runtime-types` and `workspace-contracts`
- current `core/` -> future `workspace-core`
- current renderer/main adapters -> future `workspace-adapter-*`
- current renderer UI -> future `workspace-elements`
- current React composition -> future `workspace-react` or demo host glue

## Migration Order

1. Create `sdk/` docs and workspace skeleton
2. Create `runtime-types`
3. Create `workspace-contracts`
4. Create `foundation`
5. Create `workspace-core`
6. Create adapters
7. Create design tokens
8. Create elements
9. Migrate demo to public SDK imports
10. Add React wrappers

## Prohibited Shortcuts

- moving demo directories wholesale into `sdk/`
- importing demo internals from SDK packages
- keeping reusable truth in demo after SDK package exists
- letting demo remain the only consumer path

## Cutover Rule

A capability is considered migrated only when:

- the SDK package exists for it
- demo imports the public SDK package
- demo no longer owns the reusable logic for it

## End State

Migration is complete when:

- demo can be deleted without removing SDK truth
- SDK packages depend on no demo files
- all reusable contracts and orchestration live under `sdk/`
