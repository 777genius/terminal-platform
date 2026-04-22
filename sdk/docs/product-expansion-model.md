# Product Expansion Model

**Checked**: 2026-04-22  
**Status**: frozen policy

## Goal

Define how the SDK is allowed to grow after the initial workspace UI slice exists.

## Principle

The SDK must not become one giant package or one giant domain bucket.

Growth happens by package families that reuse the same core rules.

## Initial Family

The first family is:

- `workspace-*`

This includes:

- contracts
- core
- adapters
- elements
- React layer

## Future Families

Possible future families may include:

- `transcript-*`
- `search-*`
- `diagnostics-*`
- `assistant-*`
- `settings-*`

These are examples, not commitments.

## Expansion Rule

A new family may be added only when:

- the capability is product-level and reusable
- it cannot be responsibly modeled as an internal detail of an existing family
- it follows the same dependency direction and public API discipline

## Prohibited Growth Patterns

- one mega package that absorbs every new concern
- feature growth only inside demo
- family-specific rule breaking for convenience
- new public packages with no ownership and no release policy
