# Terminal Platform UI SDK

**Checked**: 2026-04-22  
**Status**: v1 release-candidate SDK workspace

This directory contains the independent UI SDK product unit for Terminal Platform.

The SDK is intentionally separate from:

- Rust runtime truth
- host bindings such as Node and Electron leaves
- `apps/terminal-demo`, which remains a consumer only

Use these packages when embedding Terminal Platform UI in host applications:

- `@terminal-platform/runtime-types` for generated runtime mirrors.
- `@terminal-platform/workspace-contracts` and its `/commands`, `/errors`, `/observations`, and `/ports` subpaths for host-facing contracts.
- `@terminal-platform/workspace-core` and `@terminal-platform/workspace-core/testing` for the framework-neutral workspace kernel.
- `@terminal-platform/workspace-adapter-websocket`, `@terminal-platform/workspace-adapter-preload`, and `@terminal-platform/workspace-adapter-memory` for transport boundaries.
- `@terminal-platform/workspace-elements` for custom elements.
- `@terminal-platform/workspace-react` for thin React wrappers over the custom elements.
- `@terminal-platform/design-tokens/css` and `@terminal-platform/design-tokens/themes` for host-owned styling.

Read in this order:

1. [SDK Docs Index](./docs/README.md)
2. [Execution Plan](./docs/execution-plan.md)
3. [Event Model](./docs/event-model.md)
4. [Theming Model](./docs/theming-model.md)
5. [Release Policy](./docs/release-policy.md)
6. [Compatibility Matrix](./docs/compatibility-matrix.md)
7. [Package API Map](./docs/package-api-map.md)
8. [Testing Strategy](./docs/testing-strategy.md)
9. [Accessibility Model](./docs/accessibility-model.md)
10. [Keyboard And Focus Behavior](./docs/keyboard-and-focus-behavior.md)
11. [Security Model](./docs/security-model.md)
12. [Workspace Command History Model](./docs/workspace-command-history-model.md)
13. [Degraded Semantics Policy](./docs/degraded-semantics-policy.md)
14. [Diagnostics Model](./docs/diagnostics-model.md)
15. [Performance Model](./docs/performance-model.md)
16. [Product Expansion Model](./docs/product-expansion-model.md)
17. [Build And CI Model](./docs/build-and-ci-model.md)
18. [Runtime Types Generation Model](./docs/runtime-types-generation-model.md)
19. [Examples Policy](./docs/examples-policy.md)
20. [Dependency Management Policy](./docs/dependency-management-policy.md)
21. [Workspace Bootstrap Spec](./docs/workspace-bootstrap-spec.md)
22. [Package Template Spec](./docs/package-template-spec.md)
23. [Governance Model](./docs/governance-model.md)
24. [Migration Guide](./docs/migration-guide.md)
25. [ADR Set](./docs/adr/)

Core product rules:

- Rust runtime remains the only canonical terminal truth
- host bindings must not define canonical DTOs
- public SDK contracts must not leak backend-native refs
- control plane, observation plane, and screen plane stay separate
- degraded semantics must remain explicit
