# Performance Budgets

**Checked**: 2026-04-22  
**Status**: v1 preview budget artifact

## Goal

Define the minimum reviewed performance budgets required before the SDK can be called stable-ready.

These budgets are intentionally preview-level. They prevent unbounded growth and force release review, but they do not replace browser matrix profiling or host-specific performance validation.

## Budget Table

| Area | Metric | Preview Budget | Evidence Command | Notes |
| --- | --- | --- | --- | --- |
| screen-render-smoke | Render helper coverage for visible terminal output and screen action state must stay green. | `npm run test -- packages/workspace-elements/src/elements/terminal-screen-visible-output.test.ts packages/workspace-elements/src/elements/terminal-screen-actions.test.ts` | `npm run test:public-api` | Holds the DOM renderer seam until browser perf numbers are added. |
| overlay-updates | Search and command overlay presentation helpers must stay isolated from kernel mutation and keep unit coverage green. | `npm run test -- packages/workspace-elements/src/elements/terminal-screen-search-actions.test.ts packages/workspace-elements/src/elements/terminal-command-dock-accessories.test.ts` | `npm run test:public-api` | Guards overlay paths as pure presentation contracts. |
| adapter-reconnect-churn | WebSocket retry, close, and lost-close races must stay covered. | `npm run test -- packages/workspace-adapter-websocket/src/index.test.ts` | `npm run test` | Prevents reconnect churn from hanging request or subscription ownership. |
| subscription-fan-out | Kernel subscription flow and snapshot recorder coverage must stay green. | `npm run test -- packages/workspace-core/src/kernel/create-workspace-kernel.test.ts packages/workspace-core/src/testing.test.ts` | `npm run test` | Guards scoped selectors/read models until dedicated fan-out benchmarks exist. |

## Verification

Run `npm run check:performance-budgets` from `sdk/`.

The check verifies that every required area from the performance model has a non-empty metric, preview budget, evidence command, and notes field.

## Stable Release Upgrade

Before stable release, this file must be upgraded with measured browser/runtime numbers for:

- first render latency under realistic screen loads
- incremental screen delta application
- overlay update latency
- reconnect storm behavior
- subscription fan-out under multiple panes
