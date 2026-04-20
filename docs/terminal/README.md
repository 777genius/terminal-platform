# Terminal Feature Research

**Статус**: active research  
**Проверено**: 2026-04-19  
**Контекст**: новая terminal feature по [`../FEATURE_ARCHITECTURE_STANDARD.md`](../FEATURE_ARCHITECTURE_STANDARD.md)

## Зачем этот набор docs

Нужно не просто "встроить терминал", а выбрать foundation и product-паттерны для terminal UX уровня IDE:

- удобный real PTY terminal
- хороший search / selection / links / scrollback
- tabs, splits, session persistence
- нормальная интеграция с агентами, worktrees и долгоживущими задачами

Главный вывод текущего ресёрча:

⚠️ На рынке почти нет готового embeddable SDK, который из коробки даёт UX уровня Warp, Tabby, Wave или IDE terminal.  
Почти всегда приходится собирать итоговую ценность из трёх слоёв:

1. terminal foundation
2. persistence / session model
3. workspace UX поверх терминала

## Текущий baseline в этом репо

Сейчас в репо уже есть legacy/prototype terminal path:

- `package.json` уже содержит `@xterm/xterm ^6.0.0`, `@xterm/addon-fit ^0.11.0`, `@xterm/addon-web-links ^0.12.0`, `node-pty ^1.1.0`
- renderer: `src/renderer/components/terminal/EmbeddedTerminal.tsx`
- main service: `src/main/services/infrastructure/PtyTerminalService.ts`
- IPC/preload bridge: `src/main/ipc/terminal.ts`, `src/preload/index.ts`
- shared types: `src/shared/types/terminal.ts`

Что видно сразу:

- это пока не feature slice по стандарту
- PTY output идёт напрямую в `term.write(...)`, то есть backpressure/flow control не оформлен
- подключены только базовые addon-ы `fit` и `web-links`

Для новой фичи это надо рассматривать как стартовый материал, а не как финальную архитектуру.

## Главный рыночный вывод

Лучшие terminal продукты выигрывают не только renderer-ом.  
Они выигрывают комбинацией:

- shell integration
- prompt/command markers
- session persistence
- attention UX и notifications
- worktree / multi-project workflow
- search/control center
- browser/editor/task surfaces рядом с terminal

Именно поэтому рядом с foundation здесь отдельно собраны donor-проекты и infra-layer идеи.

## Текущий top 3 foundations

### 1. `wterm`
`🎯 8   🛡️ 5   🧠 5`  
Примерно `2000-4500` строк до сильного MVP.

Почему интересен:

- лучший найденный `React-friendly embed-first` путь
- есть `@wterm/react`, `@wterm/dom`, `@wterm/core`
- DOM rendering даёт native selection, browser find, accessibility, clipboard
- свежий и активно двигается

Где риск:

- проект очень молодой
- session model, persistence, shell integration и IDE UX всё равно строить самим

### 2. `restty`
`🎯 8   🛡️ 6   🧠 6`  
Примерно `2500-5000` строк до сильного MVP.

Почему интересен:

- самый "product-like" web terminal stack из свежих
- `libghostty-vt`, `WebGPU`, panes, themes, ligatures, selection
- ощущается как более мощный foundation, чем просто renderer

Где риск:

- тоже ранний проект
- React/Electron embedding story надо доводить самим

### 3. `direct libghostty path`
`🎯 7   🛡️ 5   🧠 9`  
Примерно `6000-12000` строк.

Почему интересен:

- самый сильный long-term engine path
- `libghostty-vt` уже используется многими проектами
- даёт сильную ставку на correctness и современную terminal semantics

Где риск:

- API ещё не стабилизирован
- придётся строить больше своей платформы вокруг него

## Document Map

| Файл | Что внутри |
|---|---|
| [research-foundations.md](./research-foundations.md) | Глубокий разбор `wterm`, `restty`, `ghostty-web`, `floeterm`, `libghostty` |
| [research-rust-runtime-stack.md](./research-rust-runtime-stack.md) | Rust-centric shortlist: PTY crates, emulator cores, automation layers, donor runtimes, recommended stack compositions |
| [final-v1-blueprint-rust-terminal-platform.md](./final-v1-blueprint-rust-terminal-platform.md) | Final implementation blueprint for v1: crate graph, canonical DTOs, backend ports, capability model, daemon boundary, dependency stack and staged rollout for `NativeMux` plus conservative `tmux` and `Zellij` adapters |
| [start-here-v1-implementation-pack.md](./start-here-v1-implementation-pack.md) | Main starting point for implementation: frozen decisions, reading order, workspace shape, first-week checklist, mapping to the main repo feature standard, and non-negotiable v1 rules |
| [v1-workspace-bootstrap-spec.md](./v1-workspace-bootstrap-spec.md) | Exact bootstrap source of truth for the Rust workspace: placement, root files, crate graph shape, dependency direction rules, initial module surfaces, quality gates and bootstrap definition of done |
| [v1-implementation-roadmap-and-task-breakdown.md](./v1-implementation-roadmap-and-task-breakdown.md) | Detailed execution roadmap with milestones, task buckets, deliverables, non-goals, first sprint checklist and definition of done for the implementation start phase |
| [v1-verification-and-acceptance-plan.md](./v1-verification-and-acceptance-plan.md) | Full verification plan for v1: automated layers, adapter smoke suites, manual QA checklists, acceptance gates and bug taxonomy |
| [v1-release-candidate-checklist.md](./v1-release-candidate-checklist.md) | Final ship-ready closeout checklist: canonical remote, hosted CI proof, recorded manual passes, readiness audit command, and release summary handoff for the first public v1 candidate |
| [v1-release-summary-template.md](./v1-release-summary-template.md) | Copy-ready release notes template for the first public v1 candidate: support matrix, included scope, non-promises, degraded semantics, and verification framing |
| [research-product-patterns.md](./research-product-patterns.md) | Разбор `cmux`, `Factory Floor`, `OpenCove`, `Nezha`, `Hermes IDE`, `zmx`, `termscope` |
| [research-adjacent-runtime-layers.md](./research-adjacent-runtime-layers.md) | Разбор `Mux`, `Supacode`, `Ghostree`, `Muxy`, `webterm`, `Open Terminal`, `terminalcp`, `ghostty-opentui`, `termprompt`, `ghost-complete` |
| [deep-dive-key-projects.md](./deep-dive-key-projects.md) | Deep dive по `wterm`, `restty`, `ghostty-web/libghostty`, `cmux`, `Factory Floor`, `Hermes IDE` |
| [deep-dive-session-runtime.md](./deep-dive-session-runtime.md) | Deep dive по `zmx` и `terminalcp`: attach/detach, persistence, screen vs stream, session service architecture |
| [deep-dive-code-architecture.md](./deep-dive-code-architecture.md) | Code-level patterns: transport, runtime, input, pane managers, remote daemon semantics |
| [deep-dive-product-runtime-patterns.md](./deep-dive-product-runtime-patterns.md) | Search, buffering, attention UX, resilience, automation/session protocols |
| [deep-dive-workspace-models.md](./deep-dive-workspace-models.md) | Workspace truth model, runtime modes, recovery semantics, project-aware terminal shell |
| [deep-dive-control-surface-runtime-topology.md](./deep-dive-control-surface-runtime-topology.md) | Session routes, PTY replay protocol, controller/viewer roles, runtime abstraction, remote control topology |
| [deep-dive-stream-recovery-persistence.md](./deep-dive-stream-recovery-persistence.md) | Overflow/resync, bounded replay, durable scrollback mirror, semantic analyzer layer |
| [deep-dive-foundation-resource-lifecycle.md](./deep-dive-foundation-resource-lifecycle.md) | Shared WASM/GPU/font sessions, attach ordering, per-session daemons, terminal-aware detached mode |
| [deep-dive-recovery-truth-and-intelligence.md](./deep-dive-recovery-truth-and-intelligence.md) | Owner-specific recovery truth, deferred redraw hydration, shell-aware suggestion gating, intelligence caches |
| [deep-dive-hydration-and-prompt-lifecycle.md](./deep-dive-hydration-and-prompt-lifecycle.md) | Code-level restore ordering, alt-screen hydration policy, viewport-aware output scheduling, prompt lifecycle guards |
| [deep-dive-read-models-and-semantic-runtime.md](./deep-dive-read-models-and-semantic-runtime.md) | Local find vs global search, transcript/read models, wrapped links, bounded analyzer runtime, context boundary |
| [deep-dive-control-surface-and-context-orchestration.md](./deep-dive-control-surface-and-context-orchestration.md) | Typed control surface, topology/capability negotiation, sync channel, backend phase orchestration, deferred nudge delivery |
| [deep-dive-streaming-protocol-and-timeline-persistence.md](./deep-dive-streaming-protocol-and-timeline-persistence.md) | Handshake/versioned PTY streaming, controller/viewer roles, overflow-to-snapshot recovery, raw log vs execution timeline vs context snapshots |
| [deep-dive-workstream-shell-and-adjacent-surfaces.md](./deep-dive-workstream-shell-and-adjacent-surfaces.md) | Workstream shell ownership, transparent tmux persistence, status bridge, workspace-scoped browser adjacency, remote dispatcher boundaries |
| [deep-dive-rust-runtime-architecture.md](./deep-dive-rust-runtime-architecture.md) | Source-level Rust patterns: PTY ports, emulator cores, single-thread actors, write queues, daemon control surfaces, WezTerm/Alacritty/Ghostty/Zellij lessons |
| [deep-dive-rust-embed-boundaries.md](./deep-dive-rust-embed-boundaries.md) | Public package boundaries: protocol-first design, C ABI adapters, `napi-rs` role, UniFFI/Diplomat/Interoptopus tradeoffs, packaging discipline |
| [deep-dive-rust-workspace-topologies.md](./deep-dive-rust-workspace-topologies.md) | How serious Rust terminal projects split workspaces into reusable crates, runtime/server/client leaves, adapters and frontends |
| [deep-dive-rust-mux-layout-trees-and-workspace-shell-topology.md](./deep-dive-rust-mux-layout-trees-and-workspace-shell-topology.md) | How a reusable Rust terminal runtime should model ordered windows/tabs, split-tree pane layouts, pane groups and persisted layout metadata without turning `Tab` into a god aggregate or over-generalizing the domain into a graph problem |
| [deep-dive-rust-native-tmux-zellij-multi-backend-architecture.md](./deep-dive-rust-native-tmux-zellij-multi-backend-architecture.md) | How a reusable Rust terminal platform should support `NativeMux`, `tmux` and `Zellij` simultaneously through one canonical mux contract, explicit backend families, capability negotiation and foreign-backend adapters without letting external multiplexers become the domain truth |
| [deep-dive-rust-ports-adapters-composition-roots-and-test-seams.md](./deep-dive-rust-ports-adapters-composition-roots-and-test-seams.md) | How a reusable Rust terminal runtime should express Clean Architecture in actual Rust code through explicit composition roots, narrow trait ports, async-boundary discipline, test seams and selective helper crates instead of turning DI containers or macros into the architectural center |
| [deep-dive-rust-sandboxed-extensions-wasm-plugins-and-sidecar-isolation.md](./deep-dive-rust-sandboxed-extensions-wasm-plugins-and-sidecar-isolation.md) | How a reusable Rust terminal runtime should separate host embedding, extension boundaries and sandbox policy, with concrete tradeoffs between sidecars, `wasmtime`/WIT/WASI, Extism, wRPC and lighter Wasm engines |
| [deep-dive-rust-durable-state-migrations-event-logs-and-projection-rebuilds.md](./deep-dive-rust-durable-state-migrations-event-logs-and-projection-rebuilds.md) | How a reusable Rust terminal runtime should evolve durable state over time through SQLite-centered truth, embedded migrations, append-only operational logs, versioned snapshots and rebuildable projections without confusing logs, caches and semantic timelines with real truth |
| [deep-dive-rust-state-directories-atomic-writes-locks-and-crash-consistency.md](./deep-dive-rust-state-directories-atomic-writes-locks-and-crash-consistency.md) | How a reusable Rust terminal runtime should treat state directories, atomic file publication, lock ownership, runtime socket artifacts and crash recovery as a dedicated filesystem-hygiene layer rather than hoping the chosen database or daemon transport makes these concerns disappear |
| [deep-dive-rust-daemon-ownership-leases-and-stale-recovery.md](./deep-dive-rust-daemon-ownership-leases-and-stale-recovery.md) | How a reusable Rust terminal runtime should model canonical daemon ownership, published runtime state, validate-before-reclaim logic and stale recovery using explicit lock coordination rather than pidfile folklore or app-shaped singleton helpers |
| [deep-dive-rust-daemon-readiness-version-skew-and-upgrade-handshakes.md](./deep-dive-rust-daemon-readiness-version-skew-and-upgrade-handshakes.md) | How a reusable Rust terminal runtime should make daemon readiness, version skew and upgrade behavior explicit through structured handshakes, phase-aware readiness and bounded retry tactics instead of guessing from socket reachability or startup ping loops |
| [deep-dive-rust-feature-flags-compatibility-matrix-and-capability-negotiation.md](./deep-dive-rust-feature-flags-compatibility-matrix-and-capability-negotiation.md) | How a reusable Rust terminal runtime should separate Cargo features, package compatibility policy and runtime capability negotiation, with concrete roles for `semver`, `cfg_aliases`, `document-features`, `target-lexicon`, `cargo-msrv`, `cargo-hack` and compatibility-matrix testing |
| [deep-dive-rust-runtime-primitives-and-quality.md](./deep-dive-rust-runtime-primitives-and-quality.md) | Supporting Rust stack: Tokio vs smol, framing, shutdown primitives, persistence stores, telemetry, and verification strategy |
| [deep-dive-rust-public-protocols-and-schema-evolution.md](./deep-dive-rust-public-protocols-and-schema-evolution.md) | Public contract design for a universal Rust terminal package: control plane vs data plane, protobuf/jsonrpc/capnp/flatbuffers/WIT tradeoffs, and which crates belong only inside the runtime |
| [deep-dive-rust-package-productization-and-release-discipline.md](./deep-dive-rust-package-productization-and-release-discipline.md) | What makes a Rust terminal runtime truly reusable: public error surface, config schemas, semver/API gates, supply-chain checks, process supervision, and release tooling |
| [deep-dive-rust-artifact-topology-and-multi-host-distribution.md](./deep-dive-rust-artifact-topology-and-multi-host-distribution.md) | How a reusable Rust terminal runtime should treat crates, daemon binaries, C ABI packages, Node/Electron leaves and release tooling as separate artifact families, with concrete roles for `cargo-dist`, `cargo-c`, `napi-rs`, `cargo-zigbuild`, `cargo-xwin`, `cargo-binstall` and strong packaging lessons from `rustls-ffi` |
| [deep-dive-rust-release-trust-sbom-provenance-and-platform-signing.md](./deep-dive-rust-release-trust-sbom-provenance-and-platform-signing.md) | How a reusable Rust terminal runtime should treat SBOMs, auditable binaries, dependency trust, provenance, signing and platform installer/signing lanes as outer trust architecture, with concrete roles for `cargo-auditable`, `cargo-cyclonedx`, `cargo-vet`, `cargo-about`, `sigstore`, `cosign`, `apple-codesign`, `cargo-bundle`, `cargo-deb`, `cargo-generate-rpm`, `cargo-wix` and `cargo-appimage` |
| [deep-dive-rust-workspace-publish-graph-facades-and-release-coordination.md](./deep-dive-rust-workspace-publish-graph-facades-and-release-coordination.md) | How a reusable Rust terminal runtime should distinguish internal crate modularity from public publish surfaces, use facade crates intentionally, publish host leaves separately, and coordinate multi-crate releases with tools like `release-plz`, `guppy`, `cargo-hakari`, `cargo-machete`, `cargo-deny`, `cargo-nextest` and `cargo-workspaces` |
| [deep-dive-rust-schema-type-sharing-and-host-sdk-generation.md](./deep-dive-rust-schema-type-sharing-and-host-sdk-generation.md) | How a reusable Rust terminal runtime should keep Rust protocol DTOs as truth while using targeted generators like `ts-rs`, `typeshare`, `schemars`, `typify` and `serde-reflection` for JS/UI, future multi-language host SDKs, config schemas and compatibility tooling without letting codegen become the architecture |
| [deep-dive-rust-public-api-ergonomics-builders-and-semver-shields.md](./deep-dive-rust-public-api-ergonomics-builders-and-semver-shields.md) | How a reusable Rust terminal runtime should shape its external Rust-facing API through spec objects, selective builders, sealed traits, opaque/newtype wrappers, private fields, minimal bounds and semver guardrails like `cargo-public-api` and `cargo-semver-checks` without letting builder macros become the architecture |
| [deep-dive-rust-state-machines-typestate-and-phase-modeling.md](./deep-dive-rust-state-machines-typestate-and-phase-modeling.md) | How a reusable Rust terminal runtime should model its many lifecycle and UX phases through small explicit runtime state machines, where `state-machines`, `rust-fsm`, `typestate`, `sm`, `state_machine_future` and `stateright` fit, and why verification may matter more than macro-FSM DSLs for critical invariants |
| [deep-dive-rust-public-async-streams-subscriptions-and-carriers.md](./deep-dive-rust-public-async-streams-subscriptions-and-carriers.md) | How a reusable Rust terminal runtime should expose Rust-facing async subscriptions through owned subscription objects and narrow stream adaptors, where `futures-core`, `tokio-stream`, `futures-util`, `async-stream`, `pin-project-lite`, `tokio-util` and `bytes` fit, and why internal channel types should stay out of the public contract |
| [deep-dive-rust-sync-facades-runtime-embedding-and-executor-neutrality.md](./deep-dive-rust-sync-facades-runtime-embedding-and-executor-neutrality.md) | How a reusable Rust terminal runtime should keep an async-first truth while offering optional blocking Rust facades, where `pollster`, `futures-executor`, `async-compat`, `sync_wrapper`, `maybe-async` and Tokio fit, and why blocking convenience should stay a leaf instead of dictating the public architecture |
| [deep-dive-rust-extensibility-config-and-observability.md](./deep-dive-rust-extensibility-config-and-observability.md) | How a universal Rust runtime should extend, configure, and expose telemetry: static vs dynamic plugins, config loading boundaries, hot-reload policy, and optional OpenTelemetry bridging |
| [deep-dive-rust-telemetry-metrics-profiling-and-diagnostics-boundaries.md](./deep-dive-rust-telemetry-metrics-profiling-and-diagnostics-boundaries.md) | How a reusable Rust terminal runtime should separate native semantic tracing, optional numeric metrics, opt-in profiling leaves and human-friendly diagnostics so that library crates stay clean while hosts choose subscribers, exporters and debug surfaces |
| [deep-dive-rust-state-ownership-and-handle-models.md](./deep-dive-rust-state-ownership-and-handle-models.md) | Internal entity storage vs public host-facing IDs: slotmap/generational arenas/slab, UUID vs ULID, cache boundaries, and where low-level performance types should stay hidden |
| [deep-dive-rust-ffi-handles-callbacks-and-host-sdk-boundaries.md](./deep-dive-rust-ffi-handles-callbacks-and-host-sdk-boundaries.md) | How a universal Rust terminal package should expose opaque handles, explicit event pumps/streams and host SDK seams without letting callbacks, Node adapters or binding generators become the architectural center |
| [deep-dive-rust-async-host-loops-cancellation-and-thread-affinity.md](./deep-dive-rust-async-host-loops-cancellation-and-thread-affinity.md) | How a universal Rust terminal package should cross host event loops safely: explicit operations vs subscriptions, cancellation semantics, async FFI helpers, Node thread-affinity and why promises/callbacks must stay adapter-level |
| [deep-dive-rust-ffi-memory-ownership-strings-and-buffer-boundaries.md](./deep-dive-rust-ffi-memory-ownership-strings-and-buffer-boundaries.md) | How a universal Rust terminal package should encode memory ownership across FFI: opaque handles for identity, borrowed inbound views, Rust-owned outbound blobs, string carrier choices and allocator-safe destructor rules |
| [deep-dive-rust-panic-unwind-error-envelopes-and-fault-containment.md](./deep-dive-rust-panic-unwind-error-envelopes-and-fault-containment.md) | How a universal Rust terminal package should model panic guards, unwind policy, boundary faults, session poison and runtime-fatal categories without letting panic become cross-language control flow |
| [deep-dive-rust-object-lifetime-cycles-weak-refs-and-subscription-ownership.md](./deep-dive-rust-object-lifetime-cycles-weak-refs-and-subscription-ownership.md) | How a universal Rust terminal package should keep Rust as the true owner of sessions and subscriptions, model stale handles explicitly, prevent cross-language reference cycles and keep observer edges weak instead of turning wrappers and callbacks into the real runtime graph |
| [deep-dive-rust-cross-platform-pty-backends-and-os-seams.md](./deep-dive-rust-cross-platform-pty-backends-and-os-seams.md) | How a universal Rust terminal package should keep one public PTY port while still treating Unix PTYs and Windows ConPTY as distinct infrastructure leaves with different launch, resize, cleanup and teardown semantics |
| [deep-dive-rust-unicode-width-graphemes-and-wrap-correctness.md](./deep-dive-rust-unicode-width-graphemes-and-wrap-correctness.md) | How a universal Rust terminal package should keep width, grapheme and wrap semantics in the Rust runtime so that hosts consume stable screen truth instead of independently recomputing terminal layout rules |
| [deep-dive-rust-conformance-fuzzing-and-compatibility-harnesses.md](./deep-dive-rust-conformance-fuzzing-and-compatibility-harnesses.md) | How a universal Rust terminal package should verify itself through layered property, snapshot, PTY-interaction, fuzz and external compatibility suites instead of relying on one giant test style or manual smoke tests |
| [deep-dive-rust-font-shaping-glyph-cache-and-render-boundaries.md](./deep-dive-rust-font-shaping-glyph-cache-and-render-boundaries.md) | How a universal Rust terminal package should keep terminal truth glyph-agnostic while treating font discovery, shaping, glyph caches and raster backends as optional renderer-leaf concerns |
| [deep-dive-rust-runtime-orchestration-and-message-topologies.md](./deep-dive-rust-runtime-orchestration-and-message-topologies.md) | How a reusable Rust terminal runtime should own state, supervise workers, split message lanes, and avoid turning actor frameworks or concurrent maps into accidental domain truth |
| [deep-dive-rust-parser-emulator-and-screen-boundaries.md](./deep-dive-rust-parser-emulator-and-screen-boundaries.md) | Which Rust crates belong to raw parsing, emulator core, screen snapshots/diffs, and host-side TUI shells - and why mixing them creates bad architecture for an embeddable terminal package |
| [deep-dive-rust-snapshots-replay-and-durable-formats.md](./deep-dive-rust-snapshots-replay-and-durable-formats.md) | How a reusable Rust terminal package should separate hot replay queues, durable snapshot blobs, and external tooling projections, with concrete crate choices for buffering, encoding, hashing and compression |
| [deep-dive-rust-restore-rehydrate-and-alt-screen-ordering.md](./deep-dive-rust-restore-rehydrate-and-alt-screen-ordering.md) | Why restore is an ordered runtime protocol rather than plain serialization, with concrete lessons from alacritty_terminal, vt100 and headless terminal surfaces for alt-screen, resize and reattach behavior |
| [deep-dive-rust-input-encoding-shell-integration-and-terminal-responses.md](./deep-dive-rust-input-encoding-shell-integration-and-terminal-responses.md) | How a universal Rust terminal package should model host-neutral input DTOs, terminal-state-aware encoders, shell integration protocols and terminal-generated response lanes |
| [deep-dive-rust-semantic-analysis-command-timeline-and-shell-integration-runtime.md](./deep-dive-rust-semantic-analysis-command-timeline-and-shell-integration-runtime.md) | How a universal Rust terminal package should build bounded semantic analyzers and command timelines using byte-first scanning, multi-literal detection, regex captures and narrow parser seams without turning PTY output into parser soup |
| [deep-dive-rust-pty-child-lifecycle-and-process-supervision.md](./deep-dive-rust-pty-child-lifecycle-and-process-supervision.md) | How a universal Rust terminal package should separate PTY capability from child-process supervision, using explicit group/session/job policies instead of smearing lifecycle semantics across the PTY layer |
| [deep-dive-rust-job-control-foreground-groups-and-signal-routing.md](./deep-dive-rust-job-control-foreground-groups-and-signal-routing.md) | How a reusable Rust terminal runtime should model foreground process-group ownership, typed signal intents and Unix job-control leaves without leaking `pgid`, `tcsetpgrp` or signal-plumbing details into host APIs |
| [deep-dive-rust-shell-discovery-launch-policy-and-integration-assets.md](./deep-dive-rust-shell-discovery-launch-policy-and-integration-assets.md) | How a reusable Rust terminal runtime should model shell discovery, typed launch intent, shell-like parsing boundaries and embedded shell-integration assets without letting `shell -lc` strings or helper parsers become the public process model |
| [deep-dive-rust-osc-side-effects-clipboard-notifications-and-host-bridges.md](./deep-dive-rust-osc-side-effects-clipboard-notifications-and-host-bridges.md) | How a reusable Rust terminal runtime should turn OSC-driven clipboard, hyperlink, notification and status actions into typed side-effect intents behind capability-gated host ports instead of executing platform effects directly in the core |
| [deep-dive-rust-inline-graphics-media-protocols-and-host-render-boundaries.md](./deep-dive-rust-inline-graphics-media-protocols-and-host-render-boundaries.md) | How a reusable Rust terminal runtime should treat `SIXEL`, kitty-style graphics, iTerm2-style media and host fallback rendering as optional capability leaves above text-terminal truth rather than baking protocol-specific media semantics into the core |
| [deep-dive-rust-terminal-identity-terminfo-capability-advertisement-and-env-contracts.md](./deep-dive-rust-terminal-identity-terminfo-capability-advertisement-and-env-contracts.md) | How a reusable Rust terminal runtime should separate child-visible terminal identity, env contracts, conservative capability advertisement and optional terminfo integration instead of letting host-specific env hacks define what the package claims to be |
| [deep-dive-rust-daemon-protocols-and-multi-client-topology.md](./deep-dive-rust-daemon-protocols-and-multi-client-topology.md) | How a universal Rust terminal package should structure its local daemon boundary, framed control protocol, multi-client attach semantics and outer API facades without making any one host binding or RPC framework the source of truth |
| [deep-dive-rust-local-daemon-auth-peer-credentials-and-socket-permissions.md](./deep-dive-rust-local-daemon-auth-peer-credentials-and-socket-permissions.md) | How a universal Rust terminal package should treat local daemon auth, Unix peer credentials, pathname-vs-abstract socket semantics and Windows named pipe ACL/impersonation rules as explicit authority architecture rather than ambient local trust |
| [deep-dive-rust-render-models-diffs-and-host-projections.md](./deep-dive-rust-render-models-diffs-and-host-projections.md) | How a universal Rust terminal package should expose stable render, diff and snapshot projections for any host UI without leaking emulator-specific internal grid APIs |
| [deep-dive-rust-capability-security-and-side-effect-policies.md](./deep-dive-rust-capability-security-and-side-effect-policies.md) | How a universal Rust terminal package should model filesystem authority, secrets, URL/file parsing and side-effect policies using capabilities and typed ports instead of ambient authority and stringly APIs |
| [deep-dive-rust-remote-runtime-ssh-and-transport-adapters.md](./deep-dive-rust-remote-runtime-ssh-and-transport-adapters.md) | How a universal Rust terminal package should treat SSH and remote transports as explicit runtime routes and outer adapters, with concrete tradeoffs between `wezterm-ssh`, `openssh`, `russh`, `ssh2`, and `libssh-rs` |
| [deep-dive-rust-credential-stores-ssh-agent-and-forwarding-boundaries.md](./deep-dive-rust-credential-stores-ssh-agent-and-forwarding-boundaries.md) | How a reusable Rust terminal runtime should separate stored credentials, secret material, SSH agent access, known-host policy and forwarding authority instead of collapsing them into one generic SSH-auth blob or ambient environment behavior |
| [deep-dive-rust-transcript-search-indexing-and-link-surfaces.md](./deep-dive-rust-transcript-search-indexing-and-link-surfaces.md) | How a universal Rust terminal package should separate transcript truth, rope/read-models, lightweight find, durable full-text history indices and link extraction without turning the runtime into a search engine |
| [deep-dive-rust-host-data-plane-delta-wire-contracts.md](./deep-dive-rust-host-data-plane-delta-wire-contracts.md) | How a universal Rust terminal package should separate control plane, hot data plane and durable/export encodings, and what `prost`, `flatbuffers`, `capnp`, `postcard`, `rmp-serde`, `bytes` and `tokio-util` actually mean for a host-neutral terminal SDK |
| [deep-dive-rust-zero-copy-ipc-shared-memory-and-mmap-data-paths.md](./deep-dive-rust-zero-copy-ipc-shared-memory-and-mmap-data-paths.md) | How a universal Rust terminal package should treat `mmap`, sealed `memfd`, shared-memory IPC and zero-copy ambitions as internal performance or bulk-artifact seams instead of letting shared memory become the public transport truth |
| [deep-dive-rust-backpressure-memory-budgets-and-spill-policy.md](./deep-dive-rust-backpressure-memory-budgets-and-spill-policy.md) | How a universal Rust terminal package should own output pressure, scrollback budgets and spill strategy through bounded queues and explicit memory policy instead of accidental unbounded buffering |
| [deep-dive-rust-resource-governance-quotas-timeouts-and-isolation-policies.md](./deep-dive-rust-resource-governance-quotas-timeouts-and-isolation-policies.md) | How a universal Rust terminal package should separate runtime governance from process supervision and backpressure, keep budgets and deadlines as first-class policy, and treat `rlimit`, `cgroups-rs`, and `systemd-run` as optional deployment leaves rather than core truth |
| [ideas-backlog.md](./ideas-backlog.md) | Что именно стоит утащить в нашу feature и в каком порядке |

## Промежуточный вердикт

Если бы нужно было выбирать foundation прямо сейчас:

- **самый прагматичный modern path** - `wterm`
- **самый интересный batteries-included path** - `restty`
- **самый сильный long-term bet** - `libghostty`

Но итоговый хороший продукт всё равно должен брать идеи из donor-проектов:

- `cmux` - attention UX, vertical tabs, browser split, agent notifications
- `Factory Floor` - worktrees + tmux persistence + auto port/browser workflow
- `OpenCove` - spatial workspace, global search/control center
- `Hermes IDE` - execution timeline, project-aware terminal workspace, terminal + git adjacency
- `zmx` - session persistence
- `terminalcp` - persistent session server, attach/detach, screen vs stream APIs
- `termscope` - headless automation / snapshots / CI story
- `Mux` / `Supacode` / `Ghostree` - multi-agent worktree command-center patterns
- `Trolley` / `ghostty-opentui` / `termprompt` / `ghost-complete` - runtime and UX layers around terminal

## Важный architectural rule

🔥 Terminal core должен оставаться **transparent PTY pipeline**, а "умные" фичи должны жить сверху:

- shell integration
- command blocks
- recent commands
- notifications
- task/agent metadata

И для multi-backend story теперь есть ещё одно правило:

- one canonical mux contract
- `NativeMux` as reference implementation
- `tmux` and `Zellij` as capability-gated foreign mux adapters
- no host should talk to `tmux` or `Zellij` directly

Это подтверждается и современными IDE terminal redesign-подходами, особенно JetBrains.

## Sources

- [Ghostty](https://github.com/ghostty-org/ghostty)
- [wterm](https://github.com/vercel-labs/wterm)
- [restty](https://github.com/wiedymi/restty)
- [ghostty-web](https://github.com/coder/ghostty-web)
- [floeterm](https://github.com/floegence/floeterm)
- [zmx](https://github.com/neurosnap/zmx)
- [OpenCove](https://github.com/DeadWaveWave/opencove)
- [Nezha](https://github.com/hanshuaikang/nezha)
- [Hermes IDE](https://github.com/hermes-hq/hermes-ide)
- [Factory Floor](https://github.com/alltuner/factoryfloor)
- [cmux](https://github.com/manaflow-ai/cmux)
- [zmx](https://github.com/neurosnap/zmx)
- [termscope](https://github.com/mwunsch/termscope)
- [terminalcp](https://github.com/badlogic/terminalcp)
- [Mux](https://github.com/coder/mux)
- [Supacode](https://github.com/supabitapp/supacode)
- [Ghostree](https://github.com/sidequery/ghostree)
- [Trolley](https://github.com/weedonandscott/trolley)
- [ghostty-opentui](https://github.com/remorses/ghostty-opentui)
- [termprompt](https://github.com/seeden/termprompt)
- [ghost-complete](https://github.com/StanMarek/ghost-complete)
- [JetBrains Terminal: A New Architecture](https://blog.jetbrains.com/idea/2025/04/jetbrains-terminal-a-new-architecture/)
- [VS Code Terminal Shell Integration](https://code.visualstudio.com/docs/terminal/shell-integration)
- [WezTerm Shell Integration](https://wezterm.org/shell-integration.html)
- [xterm.js Flow Control](https://xtermjs.org/docs/guides/flowcontrol/)
