# Build And CI Model

**Checked**: 2026-04-22  
**Status**: frozen policy

## Goal

Define how the UI SDK workspace is built, validated, and integrated into CI.

## Principle

The SDK must build as one disciplined workspace with shared rules, not as loosely related packages with ad hoc scripts.

## Workspace Build Model

The SDK workspace should provide:

- one root `package.json`
- one root `tsconfig.base.json`
- one shared build convention
- one shared test convention
- one shared release convention
- one explicit package manager and lockfile policy
- one ignore policy for generated workspace artifacts

## Package Template

Each package is expected to contain at minimum:

- `package.json`
- `tsconfig.json`
- `src/index.ts`

Additional files are added only when the package actually needs them.

## Output Rule

Every published package must produce:

- ESM JavaScript output
- bundled `.d.ts`
- explicit `exports`

## CI Principle

CI validates the SDK as a package graph.

It must not rely on source-linked local behavior only.

## Required CI Lanes

At minimum, the SDK CI model should include:

- workspace typecheck
- workspace build
- workspace unit tests
- browser component tests
- adapter conformance tests
- packed-consumer smoke tests

## Alignment With Repo Governance

The SDK CI model should align with the repo's existing governance style:

- formatting and consistency checks
- release discipline
- compatibility awareness

The SDK may add its own jobs, but it should not create a disconnected quality culture from the rest of the repo.

## Failure Rule

No package is considered ready for stable release if CI only passes in workspace-linked mode and fails when installed from packed artifacts.
