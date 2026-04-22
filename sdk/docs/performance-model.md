# Performance Model

**Checked**: 2026-04-22  
**Status**: frozen policy

## Goal

Define the minimum performance architecture expectations for the UI SDK.

## Principle

Performance is not a late optimization layer.

The SDK must preserve clear seams so that hot paths can evolve without rewriting product contracts.

## Critical Hot Paths

The highest-risk hot paths are:

- screen rendering
- screen delta application
- overlay updates
- subscription fan-out
- connection state churn under unstable transport

## Renderer Rule

The screen surface must keep an explicit renderer seam:

- `DomScreenRenderer` in v1
- future `CanvasScreenRenderer`
- future `WorkerRenderer`

This seam is mandatory so future perf upgrades do not require a contract rewrite.

## State Update Rule

Hot updates must not force unnecessary work across unrelated UI leaves.

The architecture must keep:

- explicit selectors
- scoped read models
- controlled subscription fan-out

## Perf Budget Rule

Stable-ready releases must define and review performance budgets for:

- screen render smoke
- overlay updates
- adapter reconnect churn
- subscription fan-out sanity

The exact numeric budgets may evolve, but the existence of budgets is mandatory.

## Anti-Pattern Rule

The following are performance architecture failures:

- monolithic screen renderer with no seam
- one generic state blob for all hot paths
- broad fan-out from adapter events to every UI leaf
- hidden expensive work in component lifecycle glue
