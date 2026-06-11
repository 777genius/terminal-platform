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

- semantic tokens, such as `--tp-color-text`, `--tp-space-4`, and `--tp-font-family-ui`
- component tokens, such as `--tp-terminal-color-bg` and `--tp-terminal-color-text`

Reference tokens may be added later, but v1 public theming starts from semantic and component CSS variables.

## Token Taxonomy

`@terminal-platform/design-tokens` exposes token taxonomy metadata:

- `terminalPlatformTokenDefinitions`
- `TERMINAL_PLATFORM_TOKEN_TIERS`
- `TERMINAL_PLATFORM_TOKEN_CATEGORIES`
- `listTerminalPlatformTokensByCategory`
- `listMissingTerminalPlatformThemeTokens`

Current categories are:

- `color`
- `terminal-color`
- `typography`
- `radius`
- `spacing`
- `elevation`

Built-in themes must provide every documented token. Host themes may add extension tokens, but extension tokens are not SDK contract unless documented.

## Token Transforms

The design token package exposes pure transforms for host integration:

- `createTerminalPlatformThemeCssDeclarations`
- `createTerminalPlatformThemeCssRule`
- `createTerminalPlatformThemeCssText`
- `listTerminalPlatformThemeTokenEntries`

Transforms order known tokens by taxonomy and append host extension tokens in stable sorted order.

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

Public workspace parts are exported as `TERMINAL_WORKSPACE_PARTS`:

- `workspace`
- `body`
- `content`
- `operations-deck`
- `terminal-column`
- `command-region`
- `inspector-column`
- `inspector-drawer`
- `navigation-drawer`
- `sidebar`
- `secondary-summary`
- `diagnostics-stack`
- `diagnostics`

Part names are public API once documented.

## Slots

Use slots for content extension only where there is a clear product-level use case.

Public workspace slots are exported as `TERMINAL_WORKSPACE_SLOTS`:

- `status-bar`
- `navigation`
- `tab-strip`
- `screen`
- `command-dock`
- `inspector`

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
