# SDK Workspace Bootstrap Spec

**Checked**: 2026-04-22  
**Status**: bootstrap source of truth

## Goal

Define the exact bootstrap expectations for the `sdk/` workspace so `Phase 1` can be implemented without guesswork.

## Placement

The UI SDK lives at:

- `/Users/belief/dev/projects/claude/terminal-platform/sdk`

This keeps it:

- structurally independent from `apps/terminal-demo`
- inside the main repo for shared delivery and coordination
- visible as a separate product unit

## Bootstrap Objectives

The initial bootstrap must produce:

- a compile-ready JS/TS workspace
- one place for shared package tooling
- explicit package boundaries from day one
- no temptation to grow reusable logic inside demo

## Required Root Files On Day One

Must exist immediately:

- `sdk/.gitignore`
- `sdk/.changeset/config.json`
- `sdk/package.json`
- `sdk/package-lock.json`
- `sdk/tsconfig.base.json`
- `sdk/vitest.config.ts`
- `sdk/README.md`
- `sdk/docs/README.md`
- `sdk/packages/*/package.json`
- `sdk/packages/*/tsconfig.json`
- `sdk/packages/*/src/index.ts`

Can wait until after bootstrap:

- package-specific test configs
- sample apps
- token build transforms
- codegen scripts beyond initial `runtime-types` setup

## Root Workspace Skeleton

```text
sdk/
  package.json
  tsconfig.base.json
  vitest.config.ts
  README.md
  docs/
  packages/
    foundation/
      package.json
      tsconfig.json
      src/index.ts
    runtime-types/
      package.json
      tsconfig.json
      src/index.ts
    design-tokens/
      package.json
      tsconfig.json
      src/index.ts
    workspace-contracts/
      package.json
      tsconfig.json
      src/index.ts
    workspace-core/
      package.json
      tsconfig.json
      src/index.ts
    workspace-adapter-websocket/
      package.json
      tsconfig.json
      src/index.ts
    workspace-adapter-preload/
      package.json
      tsconfig.json
      src/index.ts
    workspace-adapter-memory/
      package.json
      tsconfig.json
      src/index.ts
    workspace-elements/
      package.json
      tsconfig.json
      src/index.ts
    workspace-react/
      package.json
      tsconfig.json
      src/index.ts
    testing/
      package.json
      tsconfig.json
      src/index.ts
```

## Root Package Policy

The root `sdk/package.json` must provide:

- workspace package discovery
- shared scripts for build, typecheck, test, and release validation
- one package manager policy
- one changeset/release policy entrypoint
- Node engine baseline aligned with the repo policy
- workspace package ordering explicit enough to keep bootstrap deterministic

## Root TypeScript Policy

`sdk/tsconfig.base.json` must:

- define shared compiler options
- avoid framework-specific assumptions
- support declaration emit for packages
- stay boring and consistent across packages

## Initial Shared Scripts

The root workspace should expose at minimum:

- `build`
- `typecheck`
- `test`
- `check`

Where `check` runs the minimum release discipline stack for the SDK workspace.

## Bootstrap Verification Gates

Before Phase 1 is considered complete:

- all package skeletons exist
- all packages typecheck
- all packages build in empty form
- `exports` maps are already explicit
- no package imports demo code

## First Bootstrap Constraint

Do not overbuild the workspace on day one.

Bootstrap is for:

- shape
- boundaries
- quality gates

It is not for:

- full token pipeline
- full codegen sophistication
- full element library implementation
- full adapter logic
