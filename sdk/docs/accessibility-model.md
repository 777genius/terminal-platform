# Accessibility Model

**Checked**: 2026-04-22  
**Status**: frozen policy

## Goal

Define the accessibility baseline and public interaction expectations for the UI SDK.

## Principle

Accessibility is part of public API quality, not a final polish layer.

For interactive components, keyboard behavior and focus behavior are product contract.

## Core Rules

- prefer native semantics over re-creating controls with generic elements
- preserve visible focus
- define keyboard behavior explicitly
- ensure screen content, lists, and toolbars have predictable navigation
- do not let theming silently remove critical accessibility affordances

## Focus Model

The SDK must support:

- deterministic initial focus
- visible focus rings
- focus recovery after view changes
- imperative focus methods where appropriate

## Keyboard Model

Interactive public components must document:

- arrow key behavior where applicable
- enter/space behavior where applicable
- escape behavior where applicable
- tab order expectations

## Semantic Structure

The SDK should use native elements when possible for:

- buttons
- inputs
- textareas
- lists

If custom semantics are required, they must be deliberate and tested.

## Screen Surface Constraint

Terminal screen rendering may require custom handling, but supporting controls around it must still preserve:

- focus discoverability
- screen reader-safe surrounding UI
- accessible status and diagnostics messaging

## Testing Rule

Accessibility behavior must be verified in browser interaction tests, not assumed from code shape alone.
