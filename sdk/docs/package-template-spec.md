# Package Template Spec

**Checked**: 2026-04-22  
**Status**: package scaffold source of truth

## Goal

Define the minimum expected file layout and conventions for SDK packages.

## Base Template

Every SDK package starts from:

```text
<package>/
  package.json
  tsconfig.json
  src/
    index.ts
```

This is the default until the package proves it needs more structure.

## Allowed Early Extensions

Packages may add additional folders when justified:

- `src/internal/`
- `src/testing/`
- `src/generated/`
- `scripts/`

## Package Manifest Expectations

Each package manifest must include:

- package name
- version placeholder or workspace version strategy
- `"type": "module"`
- `exports`
- `types`

## TypeScript Expectations

Each `tsconfig.json` should:

- extend `sdk/tsconfig.base.json`
- set package-local `rootDir` and `outDir`
- emit declarations when publishable

## Entry Point Rule

`src/index.ts` is the public root entrypoint.

Additional entrypoints require:

- explicit `exports`
- explicit documentation in `package-api-map.md`

## Internal Code Rule

Internal folders are allowed, but internal files are not public API unless they are exported and documented.

## Package-Specific Structure Guidance

Examples:

- `runtime-types` may add `src/generated/`
- `workspace-core` may add `src/services/`, `src/reducers/`, `src/selectors/`
- `workspace-elements` may add `src/elements/`, `src/renderers/`, `src/overlays/`

## Anti-Pattern Rule

Do not create deep package trees at bootstrap just because the final system will be large.

The package template should stay minimal until the package needs more shape.
