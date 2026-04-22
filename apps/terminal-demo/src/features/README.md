# Features

This directory contains the canonical home for medium and large feature slices in `apps/terminal-demo`.

Before creating or refactoring a feature, read:
- [Feature Architecture Standard](../../docs/FEATURE_ARCHITECTURE_STANDARD.md)
- [Feature-local agent guidance](./CLAUDE.md)

Reference implementations:
- `src/features/terminal-runtime-host` - full runtime spine with `contracts/core/main/preload/renderer`
- `src/features/terminal-workspace-kernel` - shared kernel for canonical DTOs, policies, and runtime facade contracts

Use `src/features/<feature-name>/` by default when the work introduces:
- a new use case or business policy
- transport wiring
- more than one process boundary
- more than one adapter or provider

Do not duplicate architecture rules in feature folders.
Keep the standard centralized in [../../docs/FEATURE_ARCHITECTURE_STANDARD.md](../../docs/FEATURE_ARCHITECTURE_STANDARD.md).

Rule of thumb:
- `terminal-runtime-host` owns daemon, gateway, preload, bootstrap, and runtime provider composition
- capability features own only their local application and renderer slices
- `terminal-workspace-kernel/contracts` is the only allowed cross-feature dependency for internal feature code
