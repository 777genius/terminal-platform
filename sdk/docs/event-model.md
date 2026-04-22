# Event Model

**Checked**: 2026-04-22  
**Status**: frozen policy

## Goal

Define one consistent event and interaction model for the UI SDK.

## Core Principle

The SDK splits interactions into three planes:

- control plane
- observation plane
- screen plane

These planes must not collapse into one generic event bus.

## Plane Definitions

### Control Plane

Used for user and host intent:

- create session
- focus pane
- split pane
- send input
- save session

This plane is expressed through typed kernel commands, not DOM events.

### Observation Plane

Used for state, health, and topology observation:

- session catalog updates
- connection state
- capabilities
- diagnostics
- saved sessions

This plane is expressed through adapters feeding core reducers and read models.

### Screen Plane

Used for terminal screen snapshots, deltas, overlays, and render surfaces.

This plane is isolated because it is hot, high-volume, and performance-sensitive.

## Public Rules

- commands go through `WorkspaceKernel.commands`
- selectors and `getSnapshot()` expose read state
- DOM events are used only for semantic UI outputs
- screen data never becomes a generic app event stream

## DOM Event Rules

DOM events are allowed only when the host needs semantic notification from custom elements.

Examples:

- toolbar action requested
- row selected
- view requested focus

When a public DOM event is dispatched:

- it must use `CustomEvent`
- it must bubble
- it must be composed across shadow boundaries
- its payload must live in `detail`

## Prohibited Uses Of DOM Events

- carrying transport payloads
- carrying raw screen deltas
- replacing kernel commands
- mirroring internal state changes on every mutation
- dispatching events because a host merely set a property

## Naming Policy

Public DOM event names should be:

- semantic
- stable
- not tied to implementation details

Examples:

- `tp-select-session`
- `tp-run-toolbar-action`
- `tp-request-focus-input`

Avoid:

- low-level framework names
- transport names
- backend-native names

## Payload Policy

Each public event payload must:

- contain only public SDK contract types
- avoid raw transport DTOs
- avoid backend-native refs
- be minimal and purpose-specific

## Adapter Event Policy

Adapters may internally handle:

- transport frames
- transport callbacks
- reconnect hooks
- subscription events

But these stay adapter-private and are normalized before reaching core.

## Kernel Subscription Policy

The kernel owns subscriptions and fan-out:

- elements do not open backend subscriptions directly
- wrappers do not open backend subscriptions directly
- one backend subscription may feed many UI leaves through core

## Diagnostics Events

Diagnostics should be modeled as:

- read model fields
- diagnostics interfaces
- explicit logs or telemetry sinks

They should not be hidden in random event side channels.

## Versioning Rules

Changing a public DOM event name, payload shape, or dispatch guarantee is a public API change.

- additive, backward-compatible payload fields -> `MINOR`
- breaking payload or event naming changes -> `MAJOR`
