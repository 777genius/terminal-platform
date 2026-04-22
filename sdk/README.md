# UI SDK Planning Pack

**Checked**: 2026-04-22  
**Status**: architecture and execution plan frozen

This directory is the future home of the independent UI SDK product unit for Terminal Platform.

The SDK is intentionally separate from:

- Rust runtime truth
- host bindings such as Node and Electron leaves
- `apps/terminal-demo`, which remains a consumer only

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
10. [Security Model](./docs/security-model.md)
11. [Degraded Semantics Policy](./docs/degraded-semantics-policy.md)
12. [Diagnostics Model](./docs/diagnostics-model.md)
13. [Performance Model](./docs/performance-model.md)
14. [Product Expansion Model](./docs/product-expansion-model.md)
15. [Build And CI Model](./docs/build-and-ci-model.md)
16. [Runtime Types Generation Model](./docs/runtime-types-generation-model.md)
17. [Examples Policy](./docs/examples-policy.md)
18. [Dependency Management Policy](./docs/dependency-management-policy.md)
19. [Workspace Bootstrap Spec](./docs/workspace-bootstrap-spec.md)
20. [Package Template Spec](./docs/package-template-spec.md)
21. [Governance Model](./docs/governance-model.md)
22. [Migration Guide](./docs/migration-guide.md)
23. [ADR Set](./docs/adr/)

Core product rules:

- Rust runtime remains the only canonical terminal truth
- host bindings must not define canonical DTOs
- public SDK contracts must not leak backend-native refs
- control plane, observation plane, and screen plane stay separate
- degraded semantics must remain explicit
