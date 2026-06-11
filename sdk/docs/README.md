# SDK Docs Index

**Checked**: 2026-04-22  
**Status**: source of truth for UI SDK planning

This docs pack freezes the architecture and execution order for the Terminal Platform UI SDK.

The SDK is planned as an independent product unit that lives under `sdk/` and is consumed by `apps/terminal-demo`.

## Reading Order

1. [Execution Plan](./execution-plan.md)
2. [Event Model](./event-model.md)
3. [Theming Model](./theming-model.md)
4. [Release Policy](./release-policy.md)
5. [Release Checklist](./release-checklist.md)
6. [Rollback Plan](./rollback-plan.md)
7. [Deprecation Checkpoints](./deprecation-checkpoints.md)
8. [Support Policy](./support-policy.md)
9. [Compatibility Matrix](./compatibility-matrix.md)
10. [Migration Guide](./migration-guide.md)
11. [Package API Map](./package-api-map.md)
12. [Testing Strategy](./testing-strategy.md)
13. [Accessibility Model](./accessibility-model.md)
14. [Keyboard And Focus Behavior](./keyboard-and-focus-behavior.md)
15. [Security Model](./security-model.md)
16. [Workspace Command History Model](./workspace-command-history-model.md)
17. [Degraded Semantics Policy](./degraded-semantics-policy.md)
18. [Diagnostics Model](./diagnostics-model.md)
19. [Performance Model](./performance-model.md)
20. [Product Expansion Model](./product-expansion-model.md)
21. [Build And CI Model](./build-and-ci-model.md)
22. [Runtime Types Generation Model](./runtime-types-generation-model.md)
23. [Examples Policy](./examples-policy.md)
24. [Dependency Management Policy](./dependency-management-policy.md)
25. [Workspace Bootstrap Spec](./workspace-bootstrap-spec.md)
26. [Package Template Spec](./package-template-spec.md)
27. [Governance Model](./governance-model.md)
28. [Implementation Checklist](./implementation-checklist.md)
29. [ADR Set](./adr/)

## ADR Set
- [ADR Index](./adr/README.md)
- [ADR-001 - SDK Product Unit](./adr/ADR-001-sdk-product-unit.md)
- [ADR-002 - Rust Runtime Is Truth](./adr/ADR-002-rust-runtime-is-truth.md)
- [ADR-003 - Runtime Types Are Generated](./adr/ADR-003-runtime-types-are-generated.md)
- [ADR-004 - Package Graph And Dependency Direction](./adr/ADR-004-package-graph-and-dependency-direction.md)
- [ADR-005 - Headless Kernel Is Primary API](./adr/ADR-005-headless-kernel-is-primary-api.md)
- [ADR-006 - Adapters Are Anti-Corruption Layer](./adr/ADR-006-adapters-are-anti-corruption-layer.md)
- [ADR-007 - Lit Elements Are Primary UI](./adr/ADR-007-lit-elements-are-primary-ui.md)
- [ADR-008 - React Is Convenience Layer](./adr/ADR-008-react-is-convenience-layer.md)
- [ADR-009 - Event Model](./adr/ADR-009-event-model.md)
- [ADR-010 - Theming Model](./adr/ADR-010-theming-model.md)
- [ADR-011 - Publishing And SemVer](./adr/ADR-011-publishing-and-semver.md)
- [ADR-012 - Compatibility And Support Policy](./adr/ADR-012-compatibility-and-support-policy.md)
