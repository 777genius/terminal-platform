# Degraded Semantics Policy

**Checked**: 2026-04-22  
**Status**: frozen policy

## Goal

Keep degraded or partial behavior explicit across the UI SDK.

## Principle

The SDK must not fake parity across:

- Native
- tmux
- Zellij

If a capability is partial, missing, or downgraded, that fact must be visible in contracts and UI behavior.

## Contract Rule

Capability and degraded-mode data must be modeled explicitly in public contracts and read models where relevant.

## UI Rule

The UI may adapt to degraded capability, but it must not imply unsupported capability is fully available.

Examples:

- disable an action
- show reduced interaction
- show capability or degraded state messaging

## Anti-Fake-Parity Rule

Do not smooth over real backend differences by inventing silent fallback semantics that mislead the user or host integrator.

## Versioning Rule

Changing the meaning of degraded behavior in a way that affects public expectations is a public API change and must follow SemVer discipline.
