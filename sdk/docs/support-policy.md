# Support Policy

**Checked**: 2026-04-22  
**Status**: frozen policy

## Goal

Define support levels for the UI SDK across packages and environments.

## Package Maturity Labels

Each package and component must carry one label:

- `internal`
- `beta`
- `stable`
- `deprecated`

## Label Meanings

### `internal`

- not for external use
- no compatibility promise
- may change without notice

### `beta`

- intended for early adopters
- public enough for trial integration
- breaking changes still possible with explicit notice

### `stable`

- full SemVer promise
- compatibility matrix applies
- deprecation policy applies

### `deprecated`

- still available temporarily
- replacement path documented
- removal only in next `MAJOR`

## Environment Support

The SDK targets modern environments first:

- modern Chromium-based environments
- modern Safari
- modern Firefox
- Electron renderer environments

Release-grade validation should explicitly include:

- Chromium
- Firefox
- WebKit where supported by the test stack

Legacy browser support is not a v1 requirement.

## Framework Support

### First-class

- plain web/HTML/JS
- Lit element consumers
- React consumers through wrappers or custom elements directly

### Supported by documented integration

- Vue
- Angular

## Non-goals For v1 Support

- legacy browser polyfill-first support
- SSR-first guarantee
- full framework-specific UI packages beyond React wrappers

## Support Review

Support commitments must be reviewed when:

- a new stable package is introduced
- a package changes maturity label
- an environment becomes a hard product dependency
