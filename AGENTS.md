# Agent Navigation

Start here:

- Repo entrypoint: [README.md](README.md)
- Implementation pack: [docs/terminal/start-here-v1-implementation-pack.md](docs/terminal/start-here-v1-implementation-pack.md)
- Canonical blueprint: [docs/terminal/final-v1-blueprint-rust-terminal-platform.md](docs/terminal/final-v1-blueprint-rust-terminal-platform.md)
- Bootstrap spec: [docs/terminal/v1-workspace-bootstrap-spec.md](docs/terminal/v1-workspace-bootstrap-spec.md)
- Roadmap: [docs/terminal/v1-implementation-roadmap-and-task-breakdown.md](docs/terminal/v1-implementation-roadmap-and-task-breakdown.md)
- Verification plan: [docs/terminal/v1-verification-and-acceptance-plan.md](docs/terminal/v1-verification-and-acceptance-plan.md)

Non-negotiable rules:

- `NativeMux` defines the product truth
- `tmux` and `Zellij` are foreign backends
- host bindings must not define canonical DTOs
- public contracts never leak backend-native refs
- control plane and data plane stay separate
- degraded semantics must be explicit
- do not test agent teams, launch/provisioning, terminal runtime, task assignment, smoke-flow, or agent actions on real user projects
- use only new sandbox/test projects or explicitly test-only existing projects for verification
- real projects such as `~/dev/projects/ai/claude-runtime` must not be used even for opening a runtime/terminal without fresh direct user permission
