# Governance Model

**Checked**: 2026-04-22  
**Status**: frozen governance policy

## Goal

Define how the UI SDK is governed so architecture quality does not erode once implementation starts.

## Ownership Model

Ownership is explicit by package family:

- runtime truth remains owned by Rust/runtime maintainers
- `runtime-types`, `workspace-contracts`, and `workspace-core` are owned by SDK core maintainers
- `design-tokens`, `workspace-elements`, and `workspace-react` are owned by UI maintainers
- `apps/terminal-demo` is owned by app/demo maintainers

## Change Control

Public contract changes require:

- ADR update or equivalent architecture review
- compatibility matrix update when relevant
- release note impact assessment

## Foundation Growth Rule

`@terminal-platform/foundation` grows only when an abstraction is proven by repeated use.

The minimum rule is:

- at least two real consumers
- a clear repeated pattern
- a concrete benefit from centralizing it

## Demo Boundary Rule

`apps/terminal-demo` may validate SDK ergonomics, but it does not define SDK truth.

Demo convenience must not force reusable API shape.

## Review Priorities

During review, prioritize:

- dependency direction
- public API leakage
- transport coupling
- lifecycle ownership
- SemVer impact

## Architectural Escalation Triggers

Escalate review when a change:

- adds a new public package
- changes public contracts
- changes component tag names
- changes event names or event payloads
- changes theming tokens or parts
- changes compatibility guarantees
- adds a new package family

## Documentation Rule

If a public behavior changes, the change is not complete until:

- relevant docs are updated
- migration guidance is added when needed
- release policy implications are considered
