# Terminal Workspace Kernel

This feature is the shared kernel for direct terminal workspace features.

Public entrypoint:
- `@features/terminal-workspace-kernel/contracts`

Responsibilities:
- own app-defined DTOs and canonical contracts
- own pure domain policies and degraded semantics
- own runtime facade and shared state contracts consumed by capability features and runtime host
