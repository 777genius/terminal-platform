# Terminal Runtime Host

This feature is the runtime spine for `apps/terminal-demo`.

Public entrypoints:
- `@features/terminal-runtime-host/contracts`
- `@features/terminal-runtime-host/main`
- `@features/terminal-runtime-host/preload`
- `@features/terminal-runtime-host/renderer`

Responsibilities:
- own daemon supervision and gateway composition
- own Electron preload and bootstrap contracts
- own control plane and session stream adapters
- expose the renderer runtime provider and facade used by capability features
