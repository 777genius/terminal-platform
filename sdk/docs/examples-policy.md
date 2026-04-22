# Examples Policy

**Checked**: 2026-04-22  
**Status**: frozen policy

## Goal

Define the role of examples and samples in the SDK.

## Principle

Examples are downstream consumers.

They must demonstrate public SDK usage, not bypass public boundaries.

## Demo Versus Examples

`apps/terminal-demo` is:

- a showcase
- an integration lab
- a first-party consumer

Examples and samples are:

- smaller demonstrations of public SDK usage
- validation that the SDK is consumable without demo internals

## Public API Rule

Examples and samples must import only public SDK entrypoints.

They must not:

- deep-import SDK internals
- import demo internals
- depend on unpublished accidental subpaths

## Scope Rule

Examples should stay focused and narrow.

They exist to prove specific integration patterns, not to become another application shell.

## Documentation Rule

If a sample demonstrates a recommended integration pattern, the related docs should point to it explicitly.
