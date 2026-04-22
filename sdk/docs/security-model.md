# Security Model

**Checked**: 2026-04-22  
**Status**: frozen policy

## Goal

Define the trust boundary and security assumptions of the UI SDK.

## Principle

The UI SDK is not the security authority for runtime or daemon trust, but it must not silently widen the attack surface.

## Trust Boundary

The host application owns:

- daemon launch policy
- transport authentication
- transport secret distribution
- local environment trust decisions

The SDK owns:

- safe UI handling of payloads
- explicit public contracts
- avoiding accidental HTML/script interpretation

## Terminal Content Rule

Terminal output and screen payloads are treated as untrusted text data.

The SDK must not:

- interpret terminal content as trusted HTML
- inject terminal content through unsafe HTML APIs by default
- treat terminal output as sanitized markup

## Adapter Rule

Adapters must not silently weaken trust boundaries.

Examples of prohibited behavior:

- auto-connecting to untrusted endpoints without explicit host intent
- hiding transport auth assumptions inside UI components
- converting privileged host data into public UI payloads without review

## Public Contract Rule

Security-sensitive identifiers, secrets, and backend-native refs must not leak through public SDK contracts.

## Documentation Rule

If an integration path assumes specific trust or authentication behavior, it must be documented in adapter docs and release notes.
