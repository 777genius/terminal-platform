# Theming Model

**Checked**: 2026-04-22  
**Status**: frozen policy

## Goal

Define a durable theming contract for the UI SDK that:

- works across hosts and frameworks
- does not expose internal shadow DOM structure as API
- supports long-term token evolution

## Source Of Truth

The token source is DTCG-compatible and lives in `@terminal-platform/design-tokens`.

Token tiers:

- reference tokens
- semantic tokens
- component tokens

## Runtime Contract

The runtime styling contract exposed to SDK consumers is:

- CSS custom properties
- `::part()`
- slots

Nothing else is considered public styling API.

## CSS Variables

CSS variables carry:

- color semantics
- typography
- spacing
- radius
- elevation
- sizing
- terminal-specific density values where needed

Examples:

- `--tp-color-surface`
- `--tp-color-surface-muted`
- `--tp-color-text`
- `--tp-color-accent`
- `--tp-font-mono`
- `--tp-space-2`
- `--tp-radius-2`

## Parts

Expose `part` names only where structural customization is intentionally supported.

Examples:

- `header`
- `toolbar`
- `session-row`
- `screen-surface`
- `sidebar`

Part names are public API once documented.

## Slots

Use slots for content extension only where there is a clear product-level use case.

Examples:

- toolbar extra actions
- empty states
- side panels

Do not add slots for every internal node.

## Prohibited Styling Contracts

The following are not public API:

- internal shadow DOM node names
- internal class names
- internal layout wrappers
- internal Lit template structure

## Theme Scoping

Themes should be applicable:

- globally
- at subtree scope
- per host container when needed

The default mechanism for subtree scoping is CSS cascade plus host container variables, not a custom runtime theme engine.

## Relationship To Context

`@lit/context` may be used internally for theme distribution inside the element tree, but the external host contract remains CSS variables and documented slots/parts.

## Accessibility Constraint

Theming must not break:

- contrast guarantees
- focus visibility
- screen readability
- input affordances

Any theme API that can silently make core interactions inaccessible is too broad and must be reduced.

## Versioning Rules

- adding a new token or part in backward-compatible form -> `MINOR`
- removing or renaming a documented token or part -> `MAJOR`
- changing defaults without breaking documented contracts -> `PATCH` or `MINOR`, depending on visual impact
