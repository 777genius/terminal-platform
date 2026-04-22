# Runtime Types Generation Model

**Checked**: 2026-04-22  
**Status**: frozen policy

## Goal

Define how `@terminal-platform/runtime-types` is generated, owned, and kept aligned with Rust truth.

## Principle

Rust runtime remains the source of truth.

The TypeScript runtime type layer mirrors that truth. It does not redefine it.

## Ownership

Generation ownership lives with SDK core maintainers in coordination with runtime maintainers.

Neither demo nor host bindings own this package.

## Source Rule

Inputs come from canonical Rust truth and approved generation inputs only.

The package must not depend on:

- demo types
- Node leaf package types
- manually maintained duplicate truth

## Edit Rule

Generated outputs are not hand-edited.

Only:

- generation inputs
- generation scripts
- approved manual wrapper files around generated output

may be edited directly.

## Drift Rule

The workspace must include smoke checks that detect when generated TypeScript output drifts from the expected Rust source contract.

## Versioning Rule

When generation changes affect public shape, the impact must be reflected through compatibility documentation and SemVer discipline in downstream packages.
