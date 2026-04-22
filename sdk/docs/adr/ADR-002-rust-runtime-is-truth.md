# ADR-002: Rust Runtime Is Truth

**Status**: accepted  
**Date**: 2026-04-22

## Context

The project already states that Rust runtime truth is canonical and that host bindings must not define canonical DTOs.

## Decision

The UI SDK treats Rust runtime, especially `NativeMux`, as the only canonical terminal truth.

The SDK may mirror and project this truth, but it does not redefine it.

## Consequences

- UI contracts must not invent parallel runtime truth
- backend-native refs stay private
- host bindings stay leaf surfaces
- generated runtime types become mandatory
