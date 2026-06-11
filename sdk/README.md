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
- `@terminal-platform/workspace-core`, `@terminal-platform/workspace-core/bootstrap`, and `@terminal-platform/workspace-core/testing` for the framework-neutral workspace kernel and host composition helpers.
- `@terminal-platform/workspace-adapter-websocket`, `@terminal-platform/workspace-adapter-preload`, and `@terminal-platform/workspace-adapter-memory` for transport boundaries.
- `@terminal-platform/workspace-elements` for custom elements.
- `@terminal-platform/workspace-react` for thin React wrappers over the custom elements.
- `@terminal-platform/design-tokens/css` and `@terminal-platform/design-tokens/themes` for host-owned styling.
- `@terminal-platform/testing` for memory-backed workspace harnesses and packed-consumer smoke helpers in consumer integration tests.

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
16. [Performance Budgets](./docs/performance-budgets.md)
17. [Product Expansion Model](./docs/product-expansion-model.md)
18. [Build And CI Model](./docs/build-and-ci-model.md)
19. [Runtime Types Generation Model](./docs/runtime-types-generation-model.md)
20. [Examples Policy](./docs/examples-policy.md)
21. [Dependency Management Policy](./docs/dependency-management-policy.md)
22. [Workspace Bootstrap Spec](./docs/workspace-bootstrap-spec.md)
23. [Package Template Spec](./docs/package-template-spec.md)
24. [Governance Model](./docs/governance-model.md)
25. [Migration Guide](./docs/migration-guide.md)
26. [ADR Set](./docs/adr/)

For release-grade package validation, run `npm run test:packed-consumer` after the SDK build is green.

Core product rules:

- Rust runtime remains the only canonical terminal truth
- host bindings must not define canonical DTOs
- public SDK contracts must not leak backend-native refs
- control plane, observation plane, and screen plane stay separate
- degraded semantics must remain explicit

## Minimal Host Composition

```ts
import { createWorkspaceHost } from "@terminal-platform/workspace-core/bootstrap";
import { createWorkspaceWebSocketTransport } from "@terminal-platform/workspace-adapter-websocket";
import { defineTerminalPlatformElements } from "@terminal-platform/workspace-elements";

defineTerminalPlatformElements();

const host = createWorkspaceHost({
  transport: createWorkspaceWebSocketTransport({
    controlUrl: "ws://127.0.0.1:34115/workspace/control",
  }),
});

await host.bootstrap();

const workspace = document.querySelector("tp-terminal-workspace") as HTMLElement & {
  kernel: typeof host.kernel;
};

workspace.kernel = host.kernel;
```
