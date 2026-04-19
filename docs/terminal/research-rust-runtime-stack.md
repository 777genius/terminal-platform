# Rust Runtime Stack Research

**Проверено**: 2026-04-19

## Контекст

Целевая форма terminal feature теперь такая:

- UI layer на `JS/TS`
- terminal runtime/core как отдельный `Rust` project
- архитектура с явными ports/adapters, DDD-ish domain и narrow embed API для Electron

Значит главный вопрос уже не "какой terminal widget взять", а:

**какие Rust crates и готовые решения позволят не писать с нуля**

- PTY lifecycle
- terminal emulation
- screen snapshots / diffs
- automation/testing
- persistence/mux donor patterns

## Короткий вывод

🔥 Самый сильный практический вывод после Rust-прохода:

- `portable-pty` почти выглядит как default choice для PTY adapter layer
- выбор дальше упирается в emulator core:
  - `alacritty_terminal` - safest mainstream path
  - `libghostty-vt` - strongest modern long-term bet
  - `wezterm-term` / `termwiz` - powerful WezTerm ecosystem path, but with more dependency and stability caveats
- для automation и headless inspection уже есть хорошие готовые слои:
  - `expectrl`
  - `termwright`
  - `shadow-terminal`

То есть отдельный Rust terminal project вполне реально собрать **не с нуля**, а из нескольких зрелых или хотя бы полезных building blocks.

## New packaging conclusion

После более глубокого embed research стало видно ещё одно правило:

🔥 Для reusable terminal package **binding layer не должен быть главным архитектурным contract**.

Практически это значит:

- `napi-rs` - отличный Node/Electron adapter
- `safer-ffi` + `cbindgen` + `cargo-c` - сильный путь для C ABI layer
- `UniFFI` / `Diplomat` / `Interoptopus` - полезные adapter generators

Но primary truth лучше держать как:

- pure Rust runtime
- versioned protocol/control surface
- secondary adapters above that

Подробно это разобрано в [deep-dive-rust-embed-boundaries.md](./deep-dive-rust-embed-boundaries.md).

## New topology conclusion

После более глубокого workspace research стало видно ещё одно правило:

🔥 reusable terminal platform should be organized as **role-based crates**, not as one backend crate plus random helpers.

Самые полезные topology references:

- `Alacritty` - narrow reusable emulator crate
- `WezTerm` - explicit `pty / term / mux / server / gui`
- `Zellij` - explicit `client / server / plugins`
- `Rio` - reusable low-level crates plus frontend leaf

Подробно это разобрано в [deep-dive-rust-workspace-topologies.md](./deep-dive-rust-workspace-topologies.md).

## New mux and layout-topology conclusion

После ещё одного глубокого прохода стало видно, что reusable terminal runtime нельзя строить вокруг абстрактного "graph of panes".

🔥 Самый здоровый shape сейчас такой:

- ordered containers for workspace/window/tab order
- strict split-tree semantics per tab
- derived positioned geometry instead of geometry-as-truth
- `slotmap` for internal identity backbone
- pane groups as a separate bounded context
- persisted layout metadata separate from live runtime topology
- `indextree` only as an optional helper, not as the domain center
- `petgraph` and `im` only for auxiliary topology, snapshots or tooling

Подробно это разобрано в [deep-dive-rust-mux-layout-trees-and-workspace-shell-topology.md](./deep-dive-rust-mux-layout-trees-and-workspace-shell-topology.md).

## New multi-backend mux conclusion

После ещё одного глубокого прохода стало видно, что одновременная поддержка `NativeMux`, `tmux` и `Zellij` имеет смысл только если они не делят между собой архитектурную власть.

🔥 Самый здоровый shape сейчас такой:

- one canonical mux contract
- `NativeMuxBackend` as the reference and strongest implementation
- `TmuxAdapter` as a foreign mux adapter over control-mode, `capture-pane` and format subscriptions
- `ZellijAdapter` as a foreign mux adapter over `action`, `subscribe`, `dump-screen` and JSON state queries
- capability model must separate `raw_output_stream` from `rendered_viewport_stream`
- backend refs should be treated as session-scoped, not durable product IDs
- tmux tab bindings should be route-local because windows may be linked into multiple sessions
- `ZellijAdapter` should own mutation ordering through one command lane, because CLI subprocesses do not give ordering guarantees across concurrent callers
- tmux imported routes should have one explicit resize-authority policy instead of letting attach/read-only behavior decide window-size semantics accidentally
- Zellij pane refs should preserve terminal/plugin kind, and tab import should preserve backend `tab_id` separately from visual `position`
- rollout should prove canonical projections before growing foreign backend parity
- explicit split between native backends and foreign mux backends
- canonical IDs and DTOs outward, backend-specific refs inward
- capability negotiation and degraded-mode policy instead of pretending all backends are identical
- host apps always talk to our daemon/runtime, never directly to `tmux` or `Zellij`

Подробно это разобрано в [deep-dive-rust-native-tmux-zellij-multi-backend-architecture.md](./deep-dive-rust-native-tmux-zellij-multi-backend-architecture.md).

## New ports/adapters and composition-root conclusion

После ещё одного глубокого прохода стало видно, что world-class Rust runtime не стоит строить ни вокруг DI container, ни вокруг бесконтрольной россыпи trait macros.

🔥 Самый здоровый shape сейчас такой:

- explicit composition root
- constructor injection
- narrow trait ports only for real external seams
- concrete application services
- `async-trait` only at genuine dyn async boundaries
- `trait-variant` where public local/Send async trait pairs are useful
- `tower` only at outer request/response leaves
- `mockall` for tests without letting tests dictate domain shapes

Подробно это разобрано в [deep-dive-rust-ports-adapters-composition-roots-and-test-seams.md](./deep-dive-rust-ports-adapters-composition-roots-and-test-seams.md).

## New sandboxed-extension and plugin-boundary conclusion

После ещё одного глубокого прохода стало видно, что host boundary, plugin boundary и sandbox boundary нельзя смешивать.

🔥 Самый здоровый shape сейчас такой:

- v1 extensibility through built-ins and protocol-shaped sidecars
- sidecars first for risky or untrusted logic
- `wasmtime + WIT/WASI` as the strongest long-term standards-oriented sandbox path
- `Extism` only as a conscious plugin-framework decision
- `wRPC` as a later distributed component seam, not a core v1 dependency

Подробно это разобрано в [deep-dive-rust-sandboxed-extensions-wasm-plugins-and-sidecar-isolation.md](./deep-dive-rust-sandboxed-extensions-wasm-plugins-and-sidecar-isolation.md).

## New durable-state and migration-discipline conclusion

После ещё одного глубокого прохода стало видно, что durable state нельзя сводить к выбору "какую embedded DB берём".

🔥 Самый здоровый shape сейчас такой:

- SQLite as the evolving structured truth center
- `rusqlite_migration` as the default embedded migration path
- append-only operational logs kept separate from semantic timeline
- snapshot/blob versions separate from relational schema versions
- read models treated as rebuildable
- `redb` / `fjall` added only if heavier blob/replay mirrors truly justify them

Подробно это разобрано в [deep-dive-rust-durable-state-migrations-event-logs-and-projection-rebuilds.md](./deep-dive-rust-durable-state-migrations-event-logs-and-projection-rebuilds.md).

## New feature-flag and compatibility-matrix conclusion

После ещё одного глубокого прохода стало видно, что Cargo features, published compatibility policy и runtime capability negotiation нельзя считать одной и той же вещью.

🔥 Самый здоровый shape сейчас такой:

- small core crates with minimal default features
- optional heavy leaves moved to separate crates where possible
- `cfg_aliases` for cfg hygiene
- `document-features` for public feature docs
- `target-lexicon` for target-aware internal logic
- `cargo-msrv`, `cargo-hack` and `cargo-minimal-versions` as compatibility-matrix enforcement
- typed runtime handshake for build/runtime/session capabilities

Подробно это разобрано в [deep-dive-rust-feature-flags-compatibility-matrix-and-capability-negotiation.md](./deep-dive-rust-feature-flags-compatibility-matrix-and-capability-negotiation.md).

## New runtime primitives conclusion

После более глубокого supporting-stack research стало видно, что лучший default around the runtime is:

- `Tokio + tokio-util + bytes`
- `interprocess + framed local protocol`
- `CancellationToken + TaskTracker`
- `tracing`
- `rusqlite` and/or `redb`
- `loom + proptest + insta + nextest`

Подробно это разобрано в [deep-dive-rust-runtime-primitives-and-quality.md](./deep-dive-rust-runtime-primitives-and-quality.md).

## New public protocol conclusion

После ещё одного глубокого прохода стало видно, что reusable terminal package нельзя проектировать только как "Rust library + bindings".

🔥 Самый здоровый shape сейчас выглядит так:

- framed host-neutral control protocol
- separate raw byte/data plane
- optional stricter schema path через `prost`
- C ABI and host adapters как secondary boundaries
- `WIT/component model` скорее как future plugin boundary, не primary host boundary
- `rkyv` / `zerocopy` как internal performance tools, не public compatibility story

Подробно это разобрано в [deep-dive-rust-public-protocols-and-schema-evolution.md](./deep-dive-rust-public-protocols-and-schema-evolution.md).

## New package productization conclusion

После ещё одного deep dive стало видно, что reusable terminal runtime нельзя доводить до production только runtime-кодом.

🔥 Нужен ещё package discipline layer:

- `thiserror` for public library errors
- `miette` only at app/CLI leafs
- `serde + schemars` for config/manifests
- `process-wrap + signal-hook` for lifecycle/supervision
- `cargo-semver-checks + cargo-public-api` for API discipline
- `cargo-deny + cargo-audit + maybe cargo-vet` for dependency trust
- `cargo-dist + cross + cargo-zigbuild` for shipping story
- `criterion + cargo-fuzz` for credibility of hot paths

Подробно это разобрано в [deep-dive-rust-package-productization-and-release-discipline.md](./deep-dive-rust-package-productization-and-release-discipline.md).

## New artifact-topology and multi-host distribution conclusion

После ещё одного deep dive стало видно, что reusable terminal runtime нельзя мыслить как "crate, который потом как-нибудь завернём в npm".

🔥 Самый здоровый shape сейчас такой:

- protocol-first core crates as the architectural truth
- daemon binary as one explicit product surface
- C ABI package as another explicit product surface
- Node/Electron package as a leaf adapter, not as the center
- `cargo-dist` for release orchestration only
- `cargo-c + safer-ffi + cbindgen` for serious C ABI shipping
- `cargo-zigbuild` and `cargo-xwin` as different cross-build lanes
- `cargo-binstall` only as consumer convenience
- `rustls-ffi` as the strongest packaging-discipline donor

Подробно это разобрано в [deep-dive-rust-artifact-topology-and-multi-host-distribution.md](./deep-dive-rust-artifact-topology-and-multi-host-distribution.md).

## New release-trust and provenance conclusion

После ещё одного deep dive стало видно, что release trust нельзя сводить ни к semver discipline, ни к хорошему changelog, ни к platform-specific signing alone.

🔥 Самый здоровый shape сейчас такой:

- `cargo-vet` for dependency trust policy
- `cargo-about` for license inventory
- `cargo-cyclonedx` for SBOM artifacts
- `cargo-auditable` for auditable shipped binaries
- `cargo-release` and `git-cliff` as disciplined release helpers
- `sigstore` / `cosign` at the release edge
- `apple-codesign`, `cargo-bundle`, `cargo-deb`, `cargo-generate-rpm`, `cargo-wix`, `cargo-appimage` only as platform packaging leaves

Главное правило:

- trust evidence should be generated from the workspace
- signing and installer logic should stay out of core crates

Подробно это разобрано в [deep-dive-rust-release-trust-sbom-provenance-and-platform-signing.md](./deep-dive-rust-release-trust-sbom-provenance-and-platform-signing.md).

## New workspace publish-graph and facade conclusion

После ещё одного deep dive стало видно, что reusable terminal platform нельзя проектировать по принципу:

- "все полезные crates publish"

или:

- "чтобы не страдать, делаем один giant crate"

🔥 Самый здоровый shape сейчас такой:

- rich internal crate graph
- small intentional public crate graph
- facade crates as curated front doors
- separate host leaves for Node/C/daemon
- `release-plz` as the strongest release-coordination candidate
- `guppy` / `cargo-hakari` / `cargo-machete` / `cargo-deny` as workspace-graph hygiene stack

Подробно это разобрано в [deep-dive-rust-workspace-publish-graph-facades-and-release-coordination.md](./deep-dive-rust-workspace-publish-graph-facades-and-release-coordination.md).

## New schema/type-sharing and host-SDK generation conclusion

После ещё одного deep dive стало видно, что shared types нельзя проектировать как "выберем генератор и он станет нашим контрактом".

🔥 Самый здоровый shape сейчас такой:

- Rust protocol DTOs remain the intentional truth
- `ts-rs` is the strongest TS-specific default
- `typeshare` is the strongest multi-language adjacent path I found
- `schemars` stays excellent for config/manifests
- `typify` should stay an external-schema ingestion tool
- `serde-reflection` belongs more to compatibility/evolution tooling than to public SDK center

Главное правило:

- codegen should follow contracts
- contracts should not follow codegen quirks

Подробно это разобрано в [deep-dive-rust-schema-type-sharing-and-host-sdk-generation.md](./deep-dive-rust-schema-type-sharing-and-host-sdk-generation.md).

## New public-Rust-API ergonomics and semver-shield conclusion

После ещё одного deep dive стало видно, что reusable terminal package нельзя выпускать с наивным Rust-facing API и надеяться, что `cargo-semver-checks` потом всё прикроет.

🔥 Самый здоровый shape сейчас такой:

- spec objects and small entry points
- selective builder usage instead of builder-everywhere
- `typed-builder` as the strongest default builder brick
- `bon` as a strong but more worldview-shaped watchlist
- sealed traits for crate-owned extension surfaces
- private fields by default
- opaque/newtype wrappers for IDs and handles
- `cargo-public-api` and `cargo-semver-checks` as CI guardrails

Подробно это разобрано в [deep-dive-rust-public-api-ergonomics-builders-and-semver-shields.md](./deep-dive-rust-public-api-ergonomics-builders-and-semver-shields.md).

## New state-machine and typestate conclusion

После ещё одного deep dive стало видно, что для terminal runtime такого типа macro-FSM crates интересны, но чаще как bounded helpers, а не как архитектурный центр.

🔥 Самый здоровый shape сейчас такой:

- many small explicit runtime state machines
- owner-task authoritative truth
- `state-machines` as the strongest bounded runtime-FSM candidate I found
- `rust-fsm` as a respectable secondary helper
- `typestate` mostly for setup/protocol-like APIs, not for long-lived runtime truth
- `stateright` as the most interesting advanced verification seam

Главное правило:

- explicit transition logic beats elegant macro DSL when runtime is async, reconnectable and externally observable

Подробно это разобрано в [deep-dive-rust-state-machines-typestate-and-phase-modeling.md](./deep-dive-rust-state-machines-typestate-and-phase-modeling.md).

## New public-async-stream and subscription-carrier conclusion

После ещё одного deep dive стало видно, что Rust-facing async surface нельзя проектировать как "вернём Tokio receiver и готово".

🔥 Самый здоровый shape сейчас такой:

- handle-first and subscription-object-first API
- narrow `futures-core::Stream` adaptors where useful
- `tokio-stream`, `futures-util`, `async-stream` and `pin-project-lite` mostly as internal/helpers
- `bytes` for hot binary carriers
- raw-byte subscriptions and semantic-event subscriptions kept separate

Главное правило:

- stream trait is useful
- but it is not a full lifecycle model

Подробно это разобрано в [deep-dive-rust-public-async-streams-subscriptions-and-carriers.md](./deep-dive-rust-public-async-streams-subscriptions-and-carriers.md).

## New sync-facade and executor-neutrality conclusion

После ещё одного deep dive стало видно, что для reusable terminal package healthiest path - это не dual sync/async magic, а async-first truth plus optional blocking leaf.

🔥 Самый здоровый shape сейчас такой:

- canonical async Rust API
- optional separate blocking facade crate
- explicit runtime embedding inside that facade
- `pollster` only as a tactical helper
- `async-compat` and `futures-executor` only as edge helpers
- `maybe-async` not as architectural center

Главное правило:

- blocking convenience should not dictate runtime semantics

Подробно это разобрано в [deep-dive-rust-sync-facades-runtime-embedding-and-executor-neutrality.md](./deep-dive-rust-sync-facades-runtime-embedding-and-executor-neutrality.md).

## New extensibility and operations conclusion

После ещё одного прохода стало видно, что universal terminal package не стоит делать "плагинной платформой" по умолчанию.

🔥 Здоровее выглядит такой shape:

- core extensibility through typed protocol, config and feature-gated adapters
- `serde + schemars + serde_path_to_error` for host-facing config boundaries
- `tracing` as the core observability surface
- optional `tracing-opentelemetry` bridge
- optional hot-reload through immutable snapshot swap with `arc-swap`
- dynamic Rust plugin ABI only as a future product decision, not as default v1 architecture

Подробно это разобрано в [deep-dive-rust-extensibility-config-and-observability.md](./deep-dive-rust-extensibility-config-and-observability.md).

## New state and handle-model conclusion

После ещё одного глубокого прохода стало видно, что reusable terminal runtime должен очень жёстко разделять:

- public host-facing IDs
- internal storage keys
- derived read-model caches

🔥 Самый здоровый shape сейчас такой:

- `UUID` or `ULID` as opaque public handles
- `slotmap` as the strongest default for internal registries
- `generational-arena` as a simpler alternative
- `slab` only for owner-local dense tables
- `moka` only for derived caches
- `parking_lot` only as an internal primitive

Подробно это разобрано в [deep-dive-rust-state-ownership-and-handle-models.md](./deep-dive-rust-state-ownership-and-handle-models.md).

## New host SDK boundary conclusion

После ещё одного глубокого прохода стало видно, что reusable terminal package нельзя строить вокруг:

- foreign callbacks as the primary event model
- raw pointers or internal keys in public APIs
- one host binding becoming the product truth

🔥 Самый здоровый shape сейчас такой:

- opaque public handles for sessions, panes and subscriptions
- explicit event pump or framed stream as the main event boundary
- callbacks only as thin adapter sugar above that
- `safer-ffi` as the strongest C ABI path
- `napi-rs` as the best Node/Electron leaf
- `Diplomat` as the most interesting future multi-language SDK generator

Подробно это разобрано в [deep-dive-rust-ffi-handles-callbacks-and-host-sdk-boundaries.md](./deep-dive-rust-ffi-handles-callbacks-and-host-sdk-boundaries.md).

## New async host-loop and cancellation conclusion

После ещё одного глубокого прохода стало видно, что reusable terminal package не должен экспортировать host-ам один размытый async surface.

🔥 Самый здоровый shape сейчас такой:

- operations and subscriptions are different public concepts
- cancellation is explicit, not GC-driven
- adapter promises and callbacks stay above the core contract
- `async-ffi` is useful only at narrow boundaries
- `napi-rs` solves Node thread-affinity well, but only as a leaf adapter

Подробно это разобрано в [deep-dive-rust-async-host-loops-cancellation-and-thread-affinity.md](./deep-dive-rust-async-host-loops-cancellation-and-thread-affinity.md).

## New FFI memory-ownership conclusion

После ещё одного глубокого прохода стало видно, что reusable terminal package не должен скрывать ownership только в prose docs.

🔥 Самый здоровый shape сейчас такой:

- object identity through opaque handles
- inbound data as foreign-owned borrowed views
- outbound bulk data as Rust-owned blobs with explicit destructors
- multiple deliberate string carriers instead of universal `char *`
- allocator responsibility always stays with the allocating side

Подробно это разобрано в [deep-dive-rust-ffi-memory-ownership-strings-and-buffer-boundaries.md](./deep-dive-rust-ffi-memory-ownership-strings-and-buffer-boundaries.md).

## New panic and fault-containment conclusion

После ещё одного глубокого прохода стало видно, что reusable terminal package не должен смешивать domain errors, boundary faults и process-fatal conditions.

🔥 Самый здоровый shape сейчас такой:

- typed normal errors
- guarded FFI leaves
- explicit session-poison / runtime-fatal categories
- abort only on truly unsafe no-recovery edges
- future process isolation seam for genuinely dangerous islands

Подробно это разобрано в [deep-dive-rust-panic-unwind-error-envelopes-and-fault-containment.md](./deep-dive-rust-panic-unwind-error-envelopes-and-fault-containment.md).

## New object-lifetime and subscription-ownership conclusion

После ещё одного глубокого прохода стало видно, что reusable terminal package нельзя строить так, будто Rust objects и host wrappers "естественно" разделяют ownership graph.

🔥 Самый здоровый shape сейчас такой:

- Rust runtime owns the real session/subscription graph
- hosts see opaque handles and explicit owned resources
- subscriptions are first-class owned tokens with explicit close semantics
- stale handle and wrong-owner categories are deliberate public errors
- weak observer edges are used for UI/tooling/watchers instead of strong callback cycles
- generated SDK wrappers are acceptable only above this model, not instead of it

Подробно это разобрано в [deep-dive-rust-object-lifetime-cycles-weak-refs-and-subscription-ownership.md](./deep-dive-rust-object-lifetime-cycles-weak-refs-and-subscription-ownership.md).

## New cross-platform PTY and OS-seam conclusion

После ещё одного глубокого прохода стало видно, что reusable terminal package не должен путать:

- one public PTY port
- one fake identical internal PTY implementation

🔥 Самый здоровый shape сейчас такой:

- `portable-pty` remains the strongest default `PtyPort`
- Unix and Windows stay explicit infrastructure leaves
- ConPTY is treated as a real Windows architectural seam, not just "another backend"
- `rustix` looks like the healthier low-level Unix foundation if going more direct
- `pty-process` is a strong Unix donor, but not a universal core
- Windows-only crates like `conpty` and `winpty-rs` are useful donor adapters, not the center of the package

Подробно это разобрано в [deep-dive-rust-cross-platform-pty-backends-and-os-seams.md](./deep-dive-rust-cross-platform-pty-backends-and-os-seams.md).

## New Unicode width, grapheme and wrap conclusion

После ещё одного глубокого прохода стало видно, что reusable terminal package не должен отдавать host UI право решать terminal text semantics заново.

🔥 Самый здоровый shape сейчас такой:

- runtime owns width, grapheme and soft-wrap truth
- `unicode-width` is the strongest default width brick
- `unicode-segmentation` is the strongest default grapheme boundary tool
- `unicode-normalization` should stay narrow and derived
- `unicode-linebreak` and `textwrap` are useful adjacent tools, not core terminal wrap truth
- bidi-aware presentation should stay an explicit future seam, not a silent transform of live terminal state

Подробно это разобрано в [deep-dive-rust-unicode-width-graphemes-and-wrap-correctness.md](./deep-dive-rust-unicode-width-graphemes-and-wrap-correctness.md).

## New conformance, fuzzing and compatibility conclusion

После ещё одного глубокого прохода стало видно, что reusable terminal package нельзя защищать одним стилем тестов.

🔥 Самый здоровый shape сейчас такой:

- `proptest` for invariants
- `insta` for projections and golden surfaces
- `expectrl` as the strongest practical PTY interaction donor
- `termwright` as an interesting higher-level TUI testing donor
- `cargo-fuzz` for parser/protocol/replay hot paths
- `cargo-nextest` as the strongest workspace-scale runner
- `libtest-mimic` for corpus-driven compatibility suites
- `vttest` / `esctest` style external suites as explicit regression inputs

Подробно это разобрано в [deep-dive-rust-conformance-fuzzing-and-compatibility-harnesses.md](./deep-dive-rust-conformance-fuzzing-and-compatibility-harnesses.md).

## New font shaping and render-boundary conclusion

После ещё одного глубокого прохода стало видно, что reusable terminal package не должен превращать font stack в часть core runtime truth.

🔥 Самый здоровый shape сейчас такой:

- runtime stays glyph-agnostic
- `swash`, `skrifa`, `rustybuzz` and `fontdb` belong in optional renderer leaves
- `cosmic-text` is a very strong optional reference stack for standalone Rust renderers
- `parley` is a rich text layout donor for adjacent app surfaces, not core terminal truth
- `glyphon` is a backend leaf, not a host-neutral contract

Подробно это разобрано в [deep-dive-rust-font-shaping-glyph-cache-and-render-boundaries.md](./deep-dive-rust-font-shaping-glyph-cache-and-render-boundaries.md).

## New orchestration and supervision conclusion

После ещё одного глубокого прохода стало видно, что reusable terminal runtime не стоит строить вокруг:

- giant shared concurrent maps
- one generic event bus
- actor framework used everywhere by default

🔥 Самый здоровый orchestration shape сейчас такой:

- explicit owner-task runtime on `Tokio`
- separate lanes for `command`, `state`, `events` and `byte/data`
- `Tokio mpsc/oneshot/watch/broadcast` as default primitives
- `ractor` only for supervision-heavy islands
- `DashMap` / `scc` only for secondary indices, not truth ownership

Подробно это разобрано в [deep-dive-rust-runtime-orchestration-and-message-topologies.md](./deep-dive-rust-runtime-orchestration-and-message-topologies.md).

## New parser and emulator boundary conclusion

После ещё одного глубокого прохода стало видно, что Rust terminal ecosystem надо резать минимум на 4 разных responsibility layers:

- raw parser
- emulator core
- screen/diff/read-model helpers
- host-side terminal UI

🔥 Самый здоровый shape сейчас такой:

- `vte` as parser brick
- `alacritty_terminal` as default emulator core
- `vt100` / `avt` / `shadow-terminal` patterns for headless surfaces and diffs
- `crossterm` / `ratatui` only in host app shells, not in runtime truth

Подробно это разобрано в [deep-dive-rust-parser-emulator-and-screen-boundaries.md](./deep-dive-rust-parser-emulator-and-screen-boundaries.md).

## New snapshot and durable-format conclusion

После ещё одного глубокого прохода стало видно, что reusable terminal package не должен иметь один формат "на всё":

- hot replay
- durable snapshot
- export/tooling projection

🔥 Самый здоровый shape сейчас такой:

- `ringbuf` or similar bounded hot replay queue
- typed snapshot envelope encoded with something like `rmp-serde`
- optional `zstd` compression only after snapshot formation
- `blake3` for snapshot identity/dedupe
- `crc32fast` only for cheap corruption detection
- `vt100` state formatting and `shadow-terminal`-style schemas as derived projections, not core truth

Подробно это разобрано в [deep-dive-rust-snapshots-replay-and-durable-formats.md](./deep-dive-rust-snapshots-replay-and-durable-formats.md).

## New restore and alt-screen conclusion

После ещё одного глубокого прохода стало видно, что restore в serious terminal runtime нельзя мыслить как обычную десериализацию состояния.

🔥 Самый здоровый shape сейчас такой:

- committed emulator snapshot as restore truth
- explicit alternate-screen policy
- resize-aware rehydrate ordering
- bounded replay tail applied only after state and size converge
- structured external projections kept separate from internal truth

Полезные concrete signals:

- `alacritty_terminal` shows alt-screen swap as a real mode/state transition
- `vt100` explicitly maintains primary and alternate grids and can emit `state_formatted()` / `state_diff(...)`
- `shadow-terminal` shows what a rich external read-model can look like

Подробно это разобрано в [deep-dive-rust-restore-rehydrate-and-alt-screen-ordering.md](./deep-dive-rust-restore-rehydrate-and-alt-screen-ordering.md).

## New input and shell-integration conclusion

После ещё одного глубокого прохода стало видно, что reusable terminal package не должен строить input path вокруг raw host key events or UI-side escape encoding.

🔥 Самый здоровый shape сейчас такой:

- host-neutral input DTO layer
- terminal-state-aware key/mouse/paste/focus encoders in Rust
- shell integration as explicit adapter/policy layer
- terminal-generated replies on a separate lane

Practical references:

- `termwiz::input` gives one of the strongest mature input models in Rust
- `wezterm-input-types` is a good donor for host-facing key/modifier DTOs
- `libghostty-vt` already splits `key`, `mouse`, `paste`, `focus`, `osc`, and `on_pty_write`
- `alacritty_terminal` proves that encoding depends on terminal modes like bracketed paste, focus reporting, mouse modes and kitty keyboard protocol flags

Подробно это разобрано в [deep-dive-rust-input-encoding-shell-integration-and-terminal-responses.md](./deep-dive-rust-input-encoding-shell-integration-and-terminal-responses.md).

## New semantic-analyzer and command-timeline conclusion

После ещё одного глубокого прохода стало видно, что reusable terminal package не должен строить semantic runtime вокруг:

- one giant regex pile
- parser combinators over every PTY chunk
- eager conversion of all PTY bytes into `String`

🔥 Самый здоровый shape сейчас такой:

- byte-first analyzer input
- `memchr` and `aho-corasick` as cheap candidate filters
- `regex` for bounded stable capture extraction
- optional narrow parser seam like `winnow` only for structured payloads
- bounded semantic cache above raw output
- command timeline as derived projection, not transcript truth
- ANSI normalization through streaming or derived surfaces, not regex stripping

Practical references:

- `Hermes` proves the value of bounded semantic caches and provider adapter registries
- `bstr` keeps the analyzer honest about non-UTF-8 PTY realities
- `memchr` and `aho-corasick` are ideal hot-path prefilters
- `regex` remains the best default for line-local capture extraction
- `regex-automata` is the future expert upgrade, not the v1 default
- `winnow` is a good local parser seam, while `nom`/`chumsky`/`logos` are usually too heavy or misaligned as the center of this layer

Подробно это разобрано в [deep-dive-rust-semantic-analysis-command-timeline-and-shell-integration-runtime.md](./deep-dive-rust-semantic-analysis-command-timeline-and-shell-integration-runtime.md).

## New PTY child-lifecycle and process-supervision conclusion

После ещё одного глубокого прохода стало видно, что reusable terminal package нельзя проектировать так, будто PTY layer автоматически решает:

- process groups
- sessions
- job objects
- kill-on-drop policy
- detach/shutdown/reap semantics

🔥 Самый здоровый shape сейчас такой:

- `portable-pty` provides PTY capability and basic child handles
- a separate supervision adapter layer owns group/session/job policy
- runtime owner coordinates wait/kill/detach semantics
- host apps only see typed lifecycle commands and state

Important practical detail:

- `process-wrap` is an excellent supervision model and donor
- but today it looks stronger as a composable wrapper design than as a direct PTY drop-in

Supporting references:

- `portable-pty` has a very healthy `Child` / `ChildKiller` seam, including `clone_killer()`
- `process-wrap` cleanly decomposes `ProcessGroup`, `ProcessSession`, `JobObject`, `KillOnDrop`
- `signal-hook` should stay narrow and internal
- `rustix`/`nix` are strong adapter tools, not public architecture centers

Подробно это разобрано в [deep-dive-rust-pty-child-lifecycle-and-process-supervision.md](./deep-dive-rust-pty-child-lifecycle-and-process-supervision.md).

## New daemon-protocol and multi-client-topology conclusion

После ещё одного глубокого прохода стало видно, что reusable terminal package не должен делать RPC framework своим архитектурным центром.

🔥 Самый здоровый shape сейчас такой:

- local-first daemon transport on sockets/pipes
- framed control protocol
- separate PTY byte/data plane
- explicit multi-client attach semantics
- RPC and web-facing APIs only as outer facades

Practical references:

- `interprocess` is the strongest default for local daemon transport
- `tokio-util::codec` and `bytes` remain the right framing/building blocks
- `serde_json` is still a strong early control-plane format
- `prost` is the better later hardening path when contracts stabilize
- `jsonrpsee`, `tarpc`, and `tonic` each have uses, but fit better above the local runtime protocol than in its center
- `zinc`, `gritty`, and `missiond` all reinforce that attach/detach, reconnect, and daemon-owned session truth matter more than picking a flashy RPC framework

Подробно это разобрано в [deep-dive-rust-daemon-protocols-and-multi-client-topology.md](./deep-dive-rust-daemon-protocols-and-multi-client-topology.md).

## New render-model and host-projection conclusion

После ещё одного глубокого прохода стало видно, что reusable terminal package не должен отдавать host UI raw emulator internals как основной контракт.

🔥 Самый здоровый shape сейчас такой:

- emulator core remains internal truth
- runtime owns grapheme/width/wrap semantics
- live UIs consume explicit diffs or dirty-region projections
- tooling/export/reattach consume full structured snapshots

Practical references:

- `alacritty_terminal` shows the value of explicit damage tracking
- `vt100` proves that full formatted state and diffs can be first-class projections
- `shadow-terminal` proves that a host-neutral external screen schema is practical
- `libghostty-vt` shows a rich query surface, but also explicitly warns that `GridRef` is not the core render-loop API

Подробно это разобрано в [deep-dive-rust-render-models-diffs-and-host-projections.md](./deep-dive-rust-render-models-diffs-and-host-projections.md).

## New capability-security and side-effect-policy conclusion

После ещё одного глубокого прохода стало видно, что reusable terminal package нельзя делать безопасным только договором "host app всё проверит".

🔥 Самый здоровый shape сейчас такой:

- filesystem and worktree authority carried as explicit capabilities
- side effects routed through typed ports
- secrets wrapped in dedicated secret types
- URLs and paths parsed into typed values early
- shell-like string parsing kept peripheral

Practical references:

- `cap-std` is the strongest signal for capability-based file authority
- `secrecy + zeroize` form the cleanest secret-handling baseline
- `url` and `camino` help kill stringly-typed security surfaces early
- `shlex` and `shell-words` are useful helpers, but should not become the process-launch model

Подробно это разобрано в [deep-dive-rust-capability-security-and-side-effect-policies.md](./deep-dive-rust-capability-security-and-side-effect-policies.md).

## Top 3 Rust Runtime Directions

### 1. `portable-pty + alacritty_terminal + expectrl/termwright`

`🎯 9   🛡️ 9   🧠 7`  
Примерно `7000-13000` строк до хорошего reusable runtime package.

Почему это strongest pragmatic path:

- `portable-pty 0.9.0` - зрелый cross-platform PTY abstraction из экосистемы WezTerm
- `alacritty_terminal 0.26.0` от `2026-04-06` - battle-tested emulator core из Alacritty
- `expectrl 0.8.0` и `termwright 0.2.0` можно использовать как automation/test harness instead of inventing our own first

Что нравится:

- хороший баланс зрелости и контроля
- нет жёсткой завязки на один terminal product shell
- проще всего вписать в Clean Architecture как separate ports:
  - `PtyPort`
  - `EmulatorPort`
  - `AutomationPort`

Где риск:

- `alacritty_terminal` не даёт готового multiplexing/product shell
- часть современных terminal UX поверх него всё равно придётся собирать самим

### 2. `portable-pty + libghostty-vt + expectrl/termwright`

`🎯 8   🛡️ 7   🧠 8`  
Примерно `8000-15000` строк.

Почему это интересно:

- `libghostty-vt 0.1.1` от `2026-03-28` - safe Rust API поверх Ghostty terminal engine
- сам `Ghostty` прямо пишет, что `libghostty-vt` уже usable today и совместим с `macOS/Linux/Windows/WebAssembly`, но API signatures still in flux
- потенциально strongest path по correctness, Unicode, render state model и modern terminal semantics

Что нравится:

- современный engine
- уже есть `Terminal`, `RenderState`, `KeyEncoder`, `MouseEncoder`, `on_pty_write`
- хорошо выглядит как swappable emulator core для embed-first runtime

Где риск:

- ecosystem вокруг Rust bindings ещё маленький
- build story тяжелее: `Zig 0.15.x`, pinned Ghostty source, build-time fetch
- долгосрочно может быть сильнее, но short-term reliability ниже, чем у Alacritty path

### 3. `portable-pty + wezterm-term/termwiz + shadow-terminal`

`🎯 7   🛡️ 7   🧠 8`  
Примерно `8000-15000` строк.

Почему это вообще в shortlist:

- `wezterm-term` в repo WezTerm описан как actual virtual terminal emulator core
- `termwiz 0.23.3` от `2025-03-20` остаётся очень полезным adjacent toolkit:
  - surface/cell model
  - change log / delta model
  - capabilities probing
  - input decoding
- `shadow-terminal 0.2.3` даёт headless rendered terminal с structured output

Что нравится:

- вокруг WezTerm ecosystem уже есть много полезных кирпичей
- хороший donor для read models, screen deltas, headless inspection

Где риск:

- `wezterm-term` сейчас выглядит больше как git dependency from monorepo than a stable crates.io path
- сам `termwiz` в README предупреждает про active development и wild sweeping changes
- `shadow-terminal` пока больше похож на headless/test/emulation layer, чем на готовый production session runtime

## Полезные crates по слоям

## 1. PTY / process layer

### `portable-pty`

- Crate: [`portable-pty`](https://crates.io/crates/portable-pty)
- Latest: `0.9.0`
- Updated: `2025-02-11`
- Downloads: `5.3M+`
- Repo: [`wezterm/wezterm`](https://github.com/wezterm/wezterm)

Что даёт:

- `PtySystem`
- `PtyPair`
- `MasterPty`
- `SlavePty`
- `Child`
- `CommandBuilder`
- `native_pty_system()`

Почему это очень сильный кандидат:

- уже давно battle-tested inside WezTerm ecosystem
- кроссплатформенный PTY adapter слой
- shape API хорошо ложится в port-adapter architecture

Практический вывод:

✅ Я бы почти наверняка брал `portable-pty` как базовый PTY port, если только не появится очень специфическая причина использовать другой backend.

## 2. Emulator core layer

### `alacritty_terminal`

- Crate: [`alacritty_terminal`](https://crates.io/crates/alacritty_terminal)
- Latest: `0.26.0`
- Updated: `2026-04-06`
- Repo: [`alacritty/alacritty`](https://github.com/alacritty/alacritty)
- GitHub stars repo: `63.5k+`

Что даёт:

- `Term`
- optimized terminal grid
- `event_loop`
- `tty`
- `selection`
- `vi_mode`
- `vte` parser re-export

Почему это сильный default:

- очень зрелый mainstream core
- large real-world install base
- clear crate purpose: `Library for writing terminal emulators`

Главный минус:

- это именно emulator core, а не готовый embeddable runtime platform

### `libghostty-vt`

- Crate: [`libghostty-vt`](https://crates.io/crates/libghostty-vt)
- Latest: `0.1.1`
- Updated: `2026-03-28`
- Repo: [`Uzaaft/libghostty-rs`](https://github.com/Uzaaft/libghostty-rs)
- Repo stars: `254+`

Что даёт:

- safe Rust API for `libghostty-vt`
- `Terminal`
- `RenderState`
- `TerminalOptions`
- `KeyEncoder`
- `MouseEncoder`
- PTY write-back callback

Почему это интересно:

- современный engine path на базе Ghostty
- уже есть render-state model, а не только "байты на вход"

Критичный минус:

- bindings ещё молодые
- build требует `Zig`
- сам upstream `Ghostty` говорит, что API signatures ещё двигаются

### `termwiz`

- Crate: [`termwiz`](https://crates.io/crates/termwiz)
- Latest: `0.23.3`
- Updated: `2025-03-20`
- Downloads: `7.0M+`
- Repo: [`wezterm/wezterm`](https://github.com/wezterm/wezterm)

Что даёт:

- `Surface` + `Cell` model
- change log / delta apply model
- terminal capabilities probing
- keyboard/mouse decoding
- line editor and higher-level widgets

Почему полезен:

- это не просто parser, а toolbox для terminal-aware applications
- можно много reuse-ить для screen diff, state sync и capability handling

Критичный минус:

- сам README предупреждает, что crate в active development и subject to sweeping changes

### `wezterm-term`

- Source package: [`wezterm-term`](https://github.com/wezterm/wezterm/blob/main/term/Cargo.toml)
- Repo: [`wezterm/wezterm`](https://github.com/wezterm/wezterm)
- Repo stars: `25.6k+`

Что даёт:

- actual WezTerm virtual terminal emulator core
- terminal state + parser + alerts/progress/cwd/title style semantics
- no GUI and no PTY ownership by design
- `advance_bytes(...)` boundary that keeps transport separate

Почему это важно:

- architecturally this is a more direct core candidate than `termwiz`
- but the current ergonomics look more like "borrow from monorepo via git" than "clean stable crates.io dependency"

### `vt100`

- Crate: [`vt100`](https://crates.io/crates/vt100)
- Latest: `0.16.2`
- Updated: `2025-07-12`
- Downloads: `5.5M+`
- Repo: [`doy/vt100-rust`](https://github.com/doy/vt100-rust)
- Repo stars: `112+`

Что даёт:

- byte stream parser
- in-memory rendered screen
- `contents_formatted()`
- `contents_diff()` against previous screen state

Где особенно полезен:

- headless diff/snapshot use cases
- testing
- simple render/read model sidecars

Но:

- это скорее focused parser/screen crate, а не полная terminal runtime platform

## 3. Headless inspection / automation / testing

### `expectrl`

- Crate: [`expectrl`](https://crates.io/crates/expectrl)
- Latest: `0.8.0`
- Updated: `2025-09-13`
- Repo: [`zhiburt/expectrl`](https://github.com/zhiburt/expectrl)
- Repo stars: `211+`

Что даёт:

- expect-style terminal automation
- async support
- process spawn / control / expect

Почему полезен:

- не надо писать basic expect/test harness с нуля
- хороший слой для CI and automation tests against real PTYs

### `termwright`

- Crate: [`termwright`](https://crates.io/crates/termwright)
- Latest: `0.2.0`
- Updated: `2026-02-02`
- Repo: [`fcoury/termwright`](https://github.com/fcoury/termwright)
- Repo stars: `13+`

Что даёт:

- Playwright-like automation for TUIs
- PTY wrapping
- screen reading
- wait conditions
- JSON output
- PNG screenshots
- daemon mode over local Unix socket

Почему это интересно:

- очень близко к тому, что нужно для automation/inspection sidecar вокруг terminal runtime
- daemon mode особенно полезен как reference for local control protocol

Минус:

- adoption пока маленький

### `shadow-terminal`

- Crate: [`shadow-terminal`](https://crates.io/crates/shadow-terminal)
- Latest: `0.2.3`
- Updated: `2025-07-28`
- Repo: [`tattoy-org/shadow-terminal`](https://github.com/tattoy-org/shadow-terminal)
- Repo stars: `11+`

Что даёт:

- headless fully-rendered terminal emulator in memory
- `ActiveTerminal`
- `SteppableTerminal`
- JSON surface output
- built on top of WezTerm terminal core

Почему это интересно:

- это уже почти готовый headless inspection layer
- особенно полезен для structured snapshots and TUI testing

Минусы:

- roadmap openly says resize/scrollback support is still incomplete
- project пока маленький

## 4. Big donor projects, not ideal direct dependencies

### `zellij-server`

- Crate: [`zellij-server`](https://crates.io/crates/zellij-server)
- Latest: `0.44.1`
- Updated: `2026-04-07`
- Repo: [`zellij-org/zellij`](https://github.com/zellij-org/zellij)
- Repo stars: `31.7k+`

Почему важен:

- это очень сильный donor для mux/session/runtime patterns

Почему я бы не брал как base dependency:

- слишком тяжёлый server-side chunk целого продукта
- embed story слабая
- это скорее reference for multiplexer/session architecture than reusable foundation

### `par-term-emu-core-rust`

- Crate: [`par-term-emu-core-rust`](https://crates.io/crates/par-term-emu-core-rust)
- Latest: `0.41.1`
- Updated: `2026-04-11`
- Repo: [`paulrobello/par-term-emu-core-rust`](https://github.com/paulrobello/par-term-emu-core-rust)
- Repo stars: `10+`

Почему интересно:

- feature list огромный: shell integration, images, triggers, multi-session streaming server, PTY support

Почему пока risky:

- adoption очень маленький
- packaging и product direction явно сильно завязаны на Python bindings/frontends
- выглядит скорее как ambitious solo-platform than stable ecosystem foundation

## Recommended composition for our direction

Если собирать **separate Rust terminal runtime package** прагматично, я бы смотрел так:

### V1 - safest production composition

- PTY port: `portable-pty`
- emulator core: `alacritty_terminal`
- test/automation layer: `expectrl`
- richer TUI automation: `termwright`
- persistence: first `tmux` as external transparent wrapper, not custom mux from day 1

### V2 - modern engine composition

- PTY port: `portable-pty`
- emulator core: `libghostty-vt`
- test/automation layer: `expectrl` or `termwright`
- optional headless inspection donor: `shadow-terminal`

### V3 - more batteries, more churn

- PTY port: `portable-pty`
- emulator/tooling core: `wezterm-term` with `termwiz` around it
- inspection layer: `shadow-terminal` style patterns
- custom session runtime above that

## What I would avoid

- ❌ Writing your own PTY abstraction from scratch
- ❌ Writing your own ANSI/VT parser from scratch
- ❌ Building custom mux/session daemon before proving `tmux`-backed persistence
- ❌ Binding Electron directly to a pile of ad-hoc Rust functions without a typed control surface

## Architecture implication

Самый здоровый shape для отдельного Rust project сейчас выглядит так:

- `ports/pty` -> likely backed by `portable-pty`
- `ports/emulator` -> backed by `alacritty_terminal` or `libghostty-vt`
- `application/session-runtime` -> attach/detach, replay, snapshots, write queues, phases
- `application/control-surface` -> typed local protocol for Electron
- `testing/automation` -> built around `expectrl` / `termwright` style APIs

То есть reusable package должен быть не "one giant terminal crate", а layered runtime platform.

## Final v1 blueprint

После всех deep dives strongest implementation path уже можно фиксировать как canonical v1:

- `NativeMux` defines the product truth
- daemon/protocol are first-class from day one
- `tmux` and `Zellij` are foreign adapters with explicit degraded semantics
- `portable-pty + alacritty_terminal + tokio + interprocess + rusqlite` is the safest base stack
- host-facing contracts should freeze before foreign backend parity work

Итоговая реализационная схема собрана в:

- [final-v1-blueprint-rust-terminal-platform.md](./final-v1-blueprint-rust-terminal-platform.md)
- [start-here-v1-implementation-pack.md](./start-here-v1-implementation-pack.md)
- [v1-workspace-bootstrap-spec.md](./v1-workspace-bootstrap-spec.md)

## New remote-runtime and SSH conclusion

После более глубокого разбора `wezterm-ssh`, `openssh`, `russh`, `ssh2`, `libssh-rs` и `async-ssh2-lite` вывод такой:

### 1. `RemoteRoutePort` по мотивам `wezterm-ssh`
`🎯 9   🛡️ 8   🧠 8`  
Примерно `6000-12000` строк.

Это лучший shape для reusable terminal package.

Почему:

- remote должен быть explicit runtime route
- adapter island должен скрывать `libssh-rs` / `ssh2`
- наружу надо отдавать тот же `MasterPty/Child`-like seam или тот же projection contract
- `wezterm-ssh 0.4.0` уже показывает очень хороший donor shape для этого

Ограничение:

- published crate заметно отстаёт по dependency baseline, поэтому как прямую foundation dependency его надо брать осторожно

### 2. `openssh` как pragmatic outer adapter
`🎯 8   🛡️ 9   🧠 4`  
Примерно `3000-7000` строк.

Почему:

- `openssh 0.11.6` очень хорош для exec/subsystem/admin routes
- reuses system SSH config and multiplexing
- помогает быстро дать remote path без втаскивания protocol stack внутрь core runtime

Ограничение:

- Unix only
- password-less only
- плохо подходит как primary terminal truth

### 3. `russh` как deliberate pure-Rust protocol bet
`🎯 8   🛡️ 8   🧠 9`  
Примерно `8000-16000` строк.

Почему:

- `russh 0.60.0` уже выглядит как серьёзный SSH stack
- клиент, сервер, forwarding, PTY, SFTP ecosystem
- сильный long-term path, если remote станет first-class частью продукта

Ограничение:

- заметно дороже по runtime engineering
- это уже не shortcut, а осознанная platform ставка

## New transcript, search, and indexing conclusion

После более глубокого разбора `crop`, `ropey`, `tantivy`, `fst`, `linkify`, `jumprope` и `xi-rope` вывод такой:

### 1. `Append-first transcript + read-model rope + lightweight scanners`
`🎯 10   🛡️ 9   🧠 7`  
Примерно `6000-12000` строк.

Это strongest default.

Почему:

- transcript truth должен оставаться append-first
- read/search projection должен жить отдельно
- для live find хватает `memchr`, `bstr`, `aho-corasick`, `regex`
- `linkify 0.11.0` хорошо закрывает link surface
- `crop 0.4.3` сейчас выглядит лучшим stable rope candidate

### 2. `Rope-centered transcript model` через `crop` или `ropey`
`🎯 8   🛡️ 7   🧠 6`  
Примерно `5000-10000` строк.

Почему:

- ropes хорошо ложатся на line/offset/snapshot задачи
- но rope не должен становиться единственной runtime truth model
- `ropey` всё ещё силён, но latest release line сейчас `2.0.0-beta.1`

### 3. `tantivy` как separate durable history/search surface
`🎯 7   🛡️ 8   🧠 8`  
Примерно `7000-14000` строк.

Почему:

- `tantivy 0.26.0` очень силён для persisted history and global search
- но это search engine layer, а не live transcript store
- documents immutable and commit/reload semantics already signal a separate surface

## New host data-plane and wire-contract conclusion

После более глубокого разбора `prost`, `flatbuffers`, `capnp`, `rmp-serde`, `postcard`, `ciborium`, `serde_json`, `bytes` и `tokio-util` вывод такой:

### 1. `Framed control plane + explicit binary data plane`
`🎯 10   🛡️ 9   🧠 8`  
Примерно `7000-14000` строк.

Это strongest default.

Почему:

- control plane и hot data plane не должны быть одним и тем же форматом
- `bytes 1.11.1` и `tokio-util 0.7.18` хорошо ложатся на explicit framed transport
- `serde_json 1.0.149` остаётся удобным early control-plane choice
- giant JSON screen deltas - плохая идея

### 2. `Protobuf/prost for typed public envelopes + separate raw chunk lanes`
`🎯 8   🛡️ 9   🧠 8`  
Примерно `8000-15000` строк.

Почему:

- `prost 0.14.3` остаётся strong public schema candidate
- multi-language tooling story лучше, чем у serde-derived formats
- но hot screen/replay lanes всё равно лучше держать отдельно

### 3. `Zero-copy schema families` через `flatbuffers` или `capnp`
`🎯 6   🛡️ 7   🧠 9`  
Примерно `10000-18000` строк.

Почему:

- интересны для long-lived binary public contracts
- но complexity and product iteration cost выше
- сами по себе не решают правильный terminal transport shape

## New zero-copy IPC, shared-memory, and `mmap`-boundary conclusion

После более глубокого разбора `interprocess`, `memmap2`, `shared_memory`, `shmem-ipc`, `memfd`, `region`, `ringbuf` и `arc-swap` вывод такой:

### 1. `Framed socket/control plane + mmap-backed bulk artifacts`
`🎯 10   🛡️ 9   🧠 7`  
Примерно `6000-12000` строк.

Это strongest default.

Почему:

- zero-copy не должен становиться public transport philosophy
- normal daemon/session truth лучше оставлять на explicit framed protocol
- `memmap2 0.9.10` очень хорошо ложится на bulky snapshots, spill files and export artifacts
- Linux can later use `memfd 0.6.5` as an internal sealed-publication primitive without polluting the host contract

### 2. `Socket control plane + optional Linux memfd/shared-ring fast lane`
`🎯 8   🛡️ 7   🧠 9`  
Примерно `9000-17000` строк.

Почему:

- `shmem-ipc 0.3.0` даёт очень сильный donor для Linux-only acceleration
- `memfd` and shared rings are compelling for large local binary lanes
- but this should stay an explicit optional capability, not the package identity

### 3. `Shared-memory-first cross-platform transport`
`🎯 4   🛡️ 5   🧠 9`  
Примерно `8000-18000` строк.

Почему это плохой default:

- memory layout starts pretending to be protocol truth
- synchronization, stale resource cleanup and recovery leak into the core
- `shared_memory 0.12.4` looks much healthier as a low-level building block than as the center of a world-class host-neutral SDK
- JS/Electron hosts still need typed lifecycle and projection contracts anyway

Коротко:

- `memmap2` is the strongest boring default for `mmap`
- `shmem-ipc` is the strongest Linux acceleration donor
- `shared_memory` is better read as an infra primitive than as the center
- zero-copy should stay behind the runtime boundary unless proven otherwise by real profiling

## New local-daemon auth, peer-credentials, and socket-permission conclusion

После более глубокого разбора `interprocess`, `rustix`, `nix`, `uds`, `unix-cred`, `gips`, `unix(7)` и Microsoft named-pipe security docs вывод такой:

### 1. `Per-user runtime endpoint + explicit peer verification + OS-specific auth leaves`
`🎯 10   🛡️ 9   🧠 8`  
Примерно `6000-13000` строк.

Это strongest default.

Почему:

- local daemon auth should be policy-first, not path-first
- `interprocess 2.4.1` остаётся strongest cross-platform transport baseline
- Unix peer identity лучше всего выглядит через `rustix 1.1.4` и `nix 0.31.2`
- Windows named pipes требуют explicit ACL and session-scoping policy, а не default trust

### 2. `Same baseline + Unix advanced leaves for seqpacket, fd-passing and richer peer metadata`
`🎯 8   🛡️ 8   🧠 9`  
Примерно `8000-16000` строк.

Почему:

- `uds 0.4.2` даёт сильный Unix-only donor для `SOCK_SEQPACKET`, fd passing и initial peer credentials
- but these are healthier as optional Unix leaves, not as the universal control-plane contract

### 3. `Ambient trust by endpoint name, abstract sockets or default pipe security`
`🎯 3   🛡️ 3   🧠 4`  
Примерно `3000-7000` строк на старт и потом дорого чинить.

Почему это плохой default:

- Linux abstract sockets do not carry meaningful permission semantics
- `unix(7)` explicitly warns that portable programs should not rely on socket-file permissions as security
- Microsoft docs explicitly show that `NULL` named-pipe security descriptors are too permissive to treat as architecture

Коротко:

- prefer pathname sockets inside controlled runtime dirs, not abstract namespace by default
- verify peer identity explicitly on accept/attach
- keep Unix peercred and Windows named-pipe ACL/impersonation logic in dedicated OS leaves

## New daemon-ownership, lease, and stale-recovery conclusion

После более глубокого разбора `fs4`, `fd-lock`, `single-instance`, `pidlock`, `pidfile-rs`, `process_alive` и `sysinfo` вывод такой:

### 1. `Lock-first ownership + published runtime state + validated reclaim protocol`
`🎯 10   🛡️ 9   🧠 8`  
Примерно `7000-15000` строк.

Это strongest default.

Почему:

- canonical daemon ownership should be modeled as coordination lock plus validated runtime state
- `fs4 0.13.1` and `fd-lock 4.0.4` are the right class of narrow primitives
- stale reclaim should happen only after failed validation, not after seeing a socket path or pidfile alone

### 2. `Lock-first ownership + adjunct pid/liveness helpers`
`🎯 8   🛡️ 8   🧠 7`  
Примерно `5000-11000` строк.

Почему:

- `process_alive 0.2.0` is a healthy narrow adjunct for PID liveness checks
- `sysinfo 0.38.4` is useful for diagnostics and operator tooling
- but both are weaker than explicit validate-before-reclaim protocol if treated as primary truth

### 3. `Singleton helper or pidfile-first daemon ownership`
`🎯 4   🛡️ 5   🧠 5`  
Примерно `3000-8000` строк на старт и потом дорого чинить.

Почему это weak default:

- `single-instance 0.3.3` is convenient but too app-shaped and platform-divergent
- pidfile-oriented crates are useful donors, not the healthiest architecture center
- pid, lock file and endpoint existence are different signals and should not be collapsed into one accidental contract

Коротко:

- daemon ownership should be lock-first and protocol-validated
- pidfiles and singleton helpers are adjuncts or app leaves
- restart/reclaim should be modeled as ordered protocol, not startup folklore

## New daemon-readiness, version-skew, and upgrade-handshake conclusion

После более глубокого разбора `semver`, `version-compare`, `backon`, `backoff`, `tokio-retry`, `wait-timeout` и `self-replace` вывод такой:

### 1. `Explicit readiness/compatibility handshake + bounded retry helpers`
`🎯 10   🛡️ 9   🧠 8`  
Примерно `7000-15000` строк.

Это strongest default.

Почему:

- readiness should be declared, not inferred from "socket answered"
- `semver 1.0.28` stays the right compatibility brick
- `backon 1.6.0` looks like the strongest tactical retry helper, but only below an explicit handshake contract

### 2. `Same handshake + optional upgrade/reexec leaves`
`🎯 7   🛡️ 8   🧠 9`  
Примерно `9000-17000` строк.

Почему:

- `self-replace 1.5.0` is attractive for later standalone upgrade flows
- `wait-timeout 0.2.1` is useful at child-launch edges
- but upgrade choreography is later than getting readiness and skew semantics right

### 3. `Implicit startup ping loops and string-version heuristics`
`🎯 3   🛡️ 4   🧠 5`  
Примерно `3000-7000` строк на старт и потом дорого чинить.

Почему это плохой default:

- retry loops should not define what ready means
- `version-compare` is weaker than `semver` for real compatibility contracts
- "connect succeeded" is not the same thing as "daemon is compatible and ready"

Коротко:

- handshake should declare phase, protocol compatibility and capabilities
- retry crates are tactical helpers only
- upgrades and self-reexec belong to outer operational leaves
- "local" is not the same thing as "trusted"

## New backpressure, memory-budget, and spill-policy conclusion

После более глубокого разбора `thingbuf`, `crossbeam-queue`, `governor`, `ringbuf`, `memmap2`, `lru`, `mini-moka` и исходников `alacritty_terminal` вывод такой:

### 1. `Owner-task bounded lanes + explicit budgets + spill-to-disk`
`🎯 10   🛡️ 9   🧠 8`  
Примерно `7000-14000` строк.

Это strongest default.

Почему:

- backpressure должен жить в ownership model
- small hot replay can sit in `ringbuf 0.4.8`
- long-lived retention should move into durable mirror/spill layer
- unbounded buffering is architectural debt, not a convenience

### 2. `thingbuf for allocation-reusing bounded hot lanes`
`🎯 8   🛡️ 8   🧠 7`  
Примерно `5000-10000` строк.

Почему:

- `thingbuf 0.1.6` хорошо подходит для proven hot lanes
- bounded MPSC + allocation reuse is genuinely valuable
- но это должен быть narrow optimization tool, а не центр runtime

### 3. `Low-level bounded queue islands` через `ringbuf` и `crossbeam-queue`
`🎯 7   🛡️ 8   🧠 6`  
Примерно `4000-9000` строк.

Почему:

- useful for SPSC replay tails and narrow bounded MPMC islands
- but queue primitives alone do not create a good memory policy

## New resource-governance, quotas, and isolation-policy conclusion

После более глубокого разбора `tokio`, `tokio-util`, `governor`, `rlimit`, `cgroups-rs`, `systemd-run` и `process-wrap` вывод такой:

### 1. `Explicit runtime governance policy + Tokio deadlines/cancellation + optional OS leaves`
`🎯 10   🛡️ 9   🧠 8`  
Примерно `7000-15000` строк.

Это strongest default.

Почему:

- governance должен жить как typed runtime policy
- `CancellationToken`, `TaskTracker`, `Semaphore` и explicit deadlines хорошо ложатся на owner-task runtime
- `process-wrap` остаётся сильным donor для explicit supervision wrappers
- `rlimit`, `cgroups-rs`, `systemd-run` должны оставаться optional outer leaves

### 2. `Unix/Linux-heavy enforcement through rlimit + cgroups + systemd-run`
`🎯 7   🛡️ 7   🧠 8`  
Примерно `8000-16000` строк.

Почему:

- useful for controlled standalone or managed Linux deployments
- but too platform-shaped for universal embeddable default

### 3. `Helper-first governance via rate limiting and ad hoc counters`
`🎯 4   🛡️ 5   🧠 5`  
Примерно `4000-9000` строк.

Почему:

- cheap to start
- but `governor` is not the answer to session CPU/memory/time policy
- without explicit budgets governance becomes implicit and inconsistent

## New state-directory, atomic-publish, and crash-consistency conclusion

После более глубокого разбора `directories`, `camino`, `tempfile`, `atomic-write-file`, `fs4`, `fd-lock`, `interprocess`, `rusqlite` и `redb` вывод такой:

### 1. `Directories + Camino + Tempfile/atomic publish + fs4 locks + interprocess runtime artifacts`
`🎯 10   🛡️ 9   🧠 7`  
Примерно `6000-13000` строк.

Это strongest default.

Почему:

- one authority for state roots
- one explicit atomic publish path
- one visible lock strategy
- clear split between durable truth and ephemeral runtime artifacts
- crash recovery can be modeled as restart protocol instead of cleanup folklore

### 2. `Store-centric durability with minimal outer hygiene`
`🎯 6   🛡️ 6   🧠 5`  
Примерно `4000-9000` строк.

Почему:

- fast to start
- but stale sockets, partial snapshots and path drift remain under-modeled

### 3. `One giant runtime directory with ad hoc files and cleanup`
`🎯 2   🛡️ 3   🧠 4`  
Примерно `3000-7000` строк.

Почему:

- stringly paths, inconsistent rename rules and random lock behavior create long-term operational debt

## New telemetry, metrics, profiling, and diagnostics conclusion

После более глубокого разбора `tracing`, `tracing-subscriber`, `tracing-appender`, `tracing-error`, `opentelemetry`, `opentelemetry_sdk`, `metrics`, `metrics-util`, `pprof`, `console-subscriber` и `color-eyre` вывод такой:

### 1. `Tracing-first core + optional metrics + opt-in profiling leaves + leaf-only pretty diagnostics`
`🎯 10   🛡️ 9   🧠 7`  
Примерно `6000-13000` строк.

Это strongest default.

Почему:

- `tracing` remains the native semantic event surface
- numeric metrics can stay derived and optional
- profiling and pretty diagnostics stay in leaf executables and debug harnesses
- host chooses subscribers, appenders and exporters

### 2. `Tracing + metrics as parallel core surfaces, profiling mostly external`
`🎯 8   🛡️ 8   🧠 6`  
Примерно `5000-11000` строк.

Почему:

- workable if early counters truly matter
- but numeric instrumentation can start shaping architecture too early

### 3. `OTEL-first and diagnostics-heavy core`
`🎯 4   🛡️ 6   🧠 8`  
Примерно `7000-15000` строк.

Почему:

- bad universal default
- exporters and pretty diagnostics leak host assumptions into core crates

## New shell-discovery, launch-policy, and integration-asset conclusion

После более глубокого разбора `which`, `shell-words`, `shlex`, `rust-embed`, `include_dir`, `command-fds`, `portable-pty` и `process-wrap` вывод такой:

### 1. `Typed ShellLaunchSpec + discovery adapter + embedded integration assets + argv-first launch`
`🎯 10   🛡️ 9   🧠 8`  
Примерно `6000-13000` строк.

Это strongest default.

Почему:

- shell discovery stays a helper, not authority
- launch intent becomes typed instead of stringly
- integration scripts become versioned embedded assets
- helper parsers stay on product edges only

### 2. `Typed core with shell-string helpers around product edges`
`🎯 8   🛡️ 7   🧠 6`  
Примерно `5000-10000` строк.

Почему:

- useful if CLI or imported workflows explicitly accept shell syntax
- but edge convenience can easily leak inward

### 3. `String command model with shell -lc everywhere`
`🎯 3   🛡️ 4   🧠 4`  
Примерно `3000-8000` строк.

Почему:

- quoting, escaping and bootstrap logic become accidental product semantics

## New job-control, foreground-ownership, and signal-routing conclusion

После более глубокого разбора `nix`, `rustix`, `signal-hook`, `portable-pty`, `pty-process`, `wezterm` и `process-wrap` вывод такой:

### 1. `Typed JobControlPolicy + explicit foreground-owner state + Unix leaf for pgid/tcsetpgrp/killpg`
`🎯 10   🛡️ 8   🧠 9`  
Примерно `7000-15000` строк.

Это strongest default.

Почему:

- foreground ownership should be explicit runtime truth
- host should send typed intents, not raw Unix primitives
- prompt UX and shell intelligence can gate on the same truth

### 2. `Supervision-only runtime + heuristic foreground inference`
`🎯 6   🛡️ 5   🧠 5`  
Примерно `4000-9000` строк.

Почему:

- simpler to start
- but prompt/interrupt behavior becomes flaky under real TUI and shell transitions

### 3. `Expose raw pgid and signal primitives to hosts`
`🎯 3   🛡️ 4   🧠 7`  
Примерно `3000-7000` строк.

Почему:

- leaks Unix-specific internals into host adapters and weakens the cross-language embed story

## New OSC side-effects, clipboard, notification, and host-bridge conclusion

После более глубокого разбора `arboard`, `notify-rust`, `open`, `opener`, `url`, `libghostty-vt` и `cmux` notification patterns вывод такой:

### 1. `Typed side-effect intents + capability-gated host ports + separate notification/status model`
`🎯 10   🛡️ 9   🧠 7`  
Примерно `6000-13000` строк.

Это strongest default.

Почему:

- parser output can become typed intents instead of immediate side effects
- hosts keep authority over clipboard/open/notify behavior
- notifications and long-lived status stay separate runtime concepts

### 2. `Core parses intents, host UI handles most effects heuristically`
`🎯 7   🛡️ 7   🧠 5`  
Примерно `4000-9000` строк.

Почему:

- workable for one known host
- but behavior portability and capability discipline get weaker

### 3. `Immediate side effects in core runtime`
`🎯 3   🛡️ 4   🧠 4`  
Примерно `3000-7000` строк.

Почему:

- platform crates start defining runtime semantics too early

## New inline-graphics, media-protocols, and host-render-boundary conclusion

После более глубокого разбора `icy_sixel`, `sixel-rs`, `image`, `base64`, `viuer`, `ratatui-image` и `libghostty-vt` вывод такой:

### 1. `Typed media capability layer + protocol-specific optional adapters + host fallback strategies`
`🎯 10   🛡️ 8   🧠 8`  
Примерно `7000-15000` строк.

Это strongest default.

Почему:

- media support stays optional and capability-driven
- text-terminal truth remains clean
- protocol-specific adapters and fallback strategies can evolve independently

### 2. `Leaf-host widgets and viewers around a text-first runtime`
`🎯 8   🛡️ 8   🧠 6`  
Примерно `5000-10000` строк.

Почему:

- useful for standalone Rust hosts and reference integrations
- but widget crates should not define universal runtime contracts

### 3. `Media protocols inside minimal core runtime`
`🎯 3   🛡️ 4   🧠 7`  
Примерно `5000-12000` строк.

Почему:

- protocol-specific media semantics would contaminate the minimal core too early

## New terminal-identity, terminfo, capability-advertisement, and env-contract conclusion

После более глубокого разбора `terminfo`, `termini`, `terminfo-lean`, `tinf`, `termprofile` и `termwiz` вывод такой:

### 1. `Typed TerminalIdentityPolicy + explicit child env contract + optional terminfo adapter`
`🎯 10   🛡️ 9   🧠 7`  
Примерно `6000-13000` строк.

Это strongest default.

Почему:

- child-visible identity becomes explicit runtime policy
- env contract can stay stable and documented
- terminfo remains a helper/adaptation layer instead of the only truth source

### 2. `Pragmatic env-first model with limited typed policy`
`🎯 7   🛡️ 7   🧠 5`  
Примерно `4000-9000` строк.

Почему:

- workable for faster shipping
- but capability semantics drift more easily across hosts

### 3. `Host-driven identity with ad hoc env injection`
`🎯 3   🛡️ 4   🧠 4`  
Примерно `3000-7000` строк.

Почему:

- weakens compatibility guarantees and makes the package lie differently in different hosts

## New credential-store, SSH-agent, and forwarding-boundary conclusion

После более глубокого разбора `keyring`, `oo7`, `russh`, `openssh`, `openssh-mux-client`, `ssh-key`, `ssh-agent-client-rs`, `secrecy` и `zeroize` вывод такой:

### 1. `Typed CredentialProviderPort + AgentProviderPort + secret wrappers + explicit forwarding policy`
`🎯 10   🛡️ 9   🧠 8`  
Примерно `6000-13000` строк.

Это strongest default.

Почему:

- stored credentials, in-memory secrets and agent access stay separate
- forwarding becomes explicit route policy
- transport adapters consume typed capabilities instead of ambient env

### 2. `Runtime-owned OS keyring integration as primary secret store`
`🎯 7   🛡️ 7   🧠 6`  
Примерно `5000-11000` строк.

Почему:

- convenient for standalone hosts
- but cross-host neutrality weakens and the strongest crates are still `rc/alpha` on the freshest lines

### 3. `Ad hoc env/file/socket based credential handling`
`🎯 3   🛡️ 4   🧠 4`  
Примерно `3000-7000` строк.

Почему:

- agent sockets and secret material become ambient assumptions instead of explicit authority

## Sources

- [portable-pty on crates.io](https://crates.io/crates/portable-pty)
- [portable-pty docs](https://docs.rs/portable-pty/latest/portable_pty/)
- [termwiz on crates.io](https://crates.io/crates/termwiz)
- [termwiz README](https://github.com/wezterm/wezterm/blob/main/termwiz/README.md)
- [alacritty_terminal on crates.io](https://crates.io/crates/alacritty_terminal)
- [alacritty_terminal docs](https://docs.rs/alacritty_terminal/latest/alacritty_terminal/)
- [alacritty_terminal changelog](https://github.com/alacritty/alacritty/blob/master/alacritty_terminal/CHANGELOG.md)
- [Ghostty repo](https://github.com/ghostty-org/ghostty)
- [libghostty-rs](https://github.com/Uzaaft/libghostty-rs)
- [libghostty-vt on crates.io](https://crates.io/crates/libghostty-vt)
- [vt100 on crates.io](https://crates.io/crates/vt100)
- [vt100 README](https://github.com/doy/vt100-rust/blob/master/README.md)
- [expectrl](https://github.com/zhiburt/expectrl)
- [expectrl on crates.io](https://crates.io/crates/expectrl)
- [termwright](https://github.com/fcoury/termwright)
- [termwright on crates.io](https://crates.io/crates/termwright)
- [shadow-terminal](https://github.com/tattoy-org/shadow-terminal)
- [shadow-terminal on crates.io](https://crates.io/crates/shadow-terminal)
- [openssh crate](https://crates.io/crates/openssh)
- [openssh repo](https://github.com/openssh-rust/openssh)
- [russh crate](https://crates.io/crates/russh)
- [russh repo](https://github.com/Eugeny/russh)
- [ssh2 crate](https://crates.io/crates/ssh2)
- [ssh2-rs repo](https://github.com/alexcrichton/ssh2-rs)
- [wezterm-ssh crate](https://crates.io/crates/wezterm-ssh)
- [libssh-rs crate](https://crates.io/crates/libssh-rs)
- [async-ssh2-lite crate](https://crates.io/crates/async-ssh2-lite)
- [ropey crate](https://crates.io/crates/ropey)
- [ropey repo](https://github.com/cessen/ropey)
- [crop crate](https://crates.io/crates/crop)
- [crop repo](https://github.com/noib3/crop)
- [tantivy crate](https://crates.io/crates/tantivy)
- [tantivy repo](https://github.com/quickwit-oss/tantivy)
- [fst crate](https://crates.io/crates/fst)
- [fst repo](https://github.com/BurntSushi/fst)
- [linkify crate](https://crates.io/crates/linkify)
- [linkify repo](https://github.com/robinst/linkify)
- [jumprope crate](https://crates.io/crates/jumprope)
- [xi-rope crate](https://crates.io/crates/xi-rope)
- [prost crate](https://crates.io/crates/prost)
- [prost repo](https://github.com/tokio-rs/prost)
- [flatbuffers crate](https://crates.io/crates/flatbuffers)
- [flatbuffers repo](https://github.com/google/flatbuffers)
- [capnp crate](https://crates.io/crates/capnp)
- [capnproto-rust repo](https://github.com/capnproto/capnproto-rust)
- [rmp-serde crate](https://crates.io/crates/rmp-serde)
- [msgpack-rust repo](https://github.com/3Hren/msgpack-rust)
- [postcard crate](https://crates.io/crates/postcard)
- [postcard repo](https://github.com/jamesmunns/postcard)
- [ciborium crate](https://crates.io/crates/ciborium)
- [ciborium repo](https://github.com/enarx/ciborium)
- [serde_json crate](https://crates.io/crates/serde_json)
- [serde-rs/json repo](https://github.com/serde-rs/json)
- [bytes crate](https://crates.io/crates/bytes)
- [bytes repo](https://github.com/tokio-rs/bytes)
- [tokio-util crate](https://crates.io/crates/tokio-util)
- [thingbuf crate](https://crates.io/crates/thingbuf)
- [thingbuf repo](https://github.com/hawkw/thingbuf)
- [crossbeam-queue crate](https://crates.io/crates/crossbeam-queue)
- [crossbeam repo](https://github.com/crossbeam-rs/crossbeam)
- [governor crate](https://crates.io/crates/governor)
- [governor repo](https://github.com/boinkor-net/governor)
- [rlimit crate](https://crates.io/crates/rlimit)
- [rlimit repo](https://github.com/Nugine/rlimit)
- [cgroups-rs crate](https://crates.io/crates/cgroups-rs)
- [cgroups-rs repo](https://github.com/kata-containers/cgroups-rs)
- [systemd-run crate](https://crates.io/crates/systemd-run)
- [rust-systemd-run repo](https://github.com/xdu-icpc/rust-systemd-run)
- [tokio crate](https://crates.io/crates/tokio)
- [tokio repo](https://github.com/tokio-rs/tokio)
- [directories crate](https://crates.io/crates/directories)
- [directories-rs repo](https://github.com/dirs-dev/directories-rs)
- [camino crate](https://crates.io/crates/camino)
- [camino repo](https://github.com/camino-rs/camino)
- [tempfile crate](https://crates.io/crates/tempfile)
- [tempfile repo](https://github.com/Stebalien/tempfile)
- [atomic-write-file crate](https://crates.io/crates/atomic-write-file)
- [rust-atomic-write-file repo](https://github.com/andreacorbellini/rust-atomic-write-file)
- [fs-err crate](https://crates.io/crates/fs-err)
- [fs-err repo](https://github.com/andrewhickman/fs-err)
- [fs4 crate](https://crates.io/crates/fs4)
- [fs4-rs repo](https://github.com/al8n/fs4-rs)
- [fd-lock crate](https://crates.io/crates/fd-lock)
- [fd-lock repo](https://github.com/yoshuawuyts/fd-lock)
- [tracing crate](https://crates.io/crates/tracing)
- [tokio-rs/tracing](https://github.com/tokio-rs/tracing)
- [tracing-subscriber crate](https://crates.io/crates/tracing-subscriber)
- [tracing-appender crate](https://crates.io/crates/tracing-appender)
- [tracing-error crate](https://crates.io/crates/tracing-error)
- [opentelemetry crate](https://crates.io/crates/opentelemetry)
- [opentelemetry_sdk crate](https://crates.io/crates/opentelemetry_sdk)
- [OpenTelemetry Rust](https://github.com/open-telemetry/opentelemetry-rust)
- [metrics crate](https://crates.io/crates/metrics)
- [metrics-util crate](https://crates.io/crates/metrics-util)
- [metrics-rs/metrics](https://github.com/metrics-rs/metrics)
- [pprof crate](https://crates.io/crates/pprof)
- [tikv/pprof-rs](https://github.com/tikv/pprof-rs)
- [console-subscriber crate](https://crates.io/crates/console-subscriber)
- [tokio-rs/console](https://github.com/tokio-rs/console)
- [color-eyre crate](https://crates.io/crates/color-eyre)
- [eyre-rs/eyre](https://github.com/eyre-rs/eyre)
- [which crate](https://crates.io/crates/which)
- [which-rs repo](https://github.com/harryfei/which-rs)
- [shell-words crate](https://crates.io/crates/shell-words)
- [shell-words repo](https://github.com/tmiasko/shell-words)
- [shlex crate](https://crates.io/crates/shlex)
- [shlex repo](https://github.com/comex/rust-shlex)
- [rust-embed crate](https://crates.io/crates/rust-embed)
- [include_dir crate](https://crates.io/crates/include_dir)
- [include_dir repo](https://github.com/Michael-F-Bryan/include_dir)
- [command-fds crate](https://crates.io/crates/command-fds)
- [command-fds repo](https://github.com/google/command-fds)
- [nix crate](https://crates.io/crates/nix)
- [nix repo](https://github.com/nix-rust/nix)
- [rustix crate](https://crates.io/crates/rustix)
- [rustix repo](https://github.com/bytecodealliance/rustix)
- [signal-hook crate](https://crates.io/crates/signal-hook)
- [signal-hook repo](https://github.com/vorner/signal-hook)
- [arboard crate](https://crates.io/crates/arboard)
- [1Password/arboard](https://github.com/1Password/arboard)
- [notify-rust crate](https://crates.io/crates/notify-rust)
- [hoodie/notify-rust](https://github.com/hoodie/notify-rust)
- [open crate](https://crates.io/crates/open)
- [Byron/open-rs](https://github.com/Byron/open-rs)
- [opener crate](https://crates.io/crates/opener)
- [Seeker14491/opener](https://github.com/Seeker14491/opener)
- [url crate](https://crates.io/crates/url)
- [servo/rust-url](https://github.com/servo/rust-url)
- [libghostty-vt crate](https://crates.io/crates/libghostty-vt)
- [Uzaaft/libghostty-rs](https://github.com/Uzaaft/libghostty-rs)
- [icy_sixel crate](https://crates.io/crates/icy_sixel)
- [mkrueger/icy_sixel](https://github.com/mkrueger/icy_sixel)
- [sixel-rs crate](https://crates.io/crates/sixel-rs)
- [image crate](https://crates.io/crates/image)
- [image-rs/image](https://github.com/image-rs/image)
- [base64 crate](https://crates.io/crates/base64)
- [viuer crate](https://crates.io/crates/viuer)
- [atanunq/viuer](https://github.com/atanunq/viuer)
- [ratatui-image crate](https://crates.io/crates/ratatui-image)
- [ratatui/ratatui-image](https://github.com/ratatui/ratatui-image)
- [terminfo crate](https://crates.io/crates/terminfo)
- [meh/rust-terminfo](https://github.com/meh/rust-terminfo)
- [termini crate](https://crates.io/crates/termini)
- [pascalkuthe/termini](https://github.com/pascalkuthe/termini)
- [terminfo-lean crate](https://crates.io/crates/terminfo-lean)
- [proski/terminfo-lean](https://github.com/proski/terminfo-lean)
- [tinf crate](https://crates.io/crates/tinf)
- [edmccard/tvis](https://github.com/edmccard/tvis)
- [termprofile crate](https://crates.io/crates/termprofile)
- [aschey/termprofile](https://github.com/aschey/termprofile)
- [termwiz crate](https://crates.io/crates/termwiz)
- [wezterm/wezterm](https://github.com/wezterm/wezterm)
- [keyring crate](https://crates.io/crates/keyring)
- [open-source-cooperative/keyring-rs](https://github.com/open-source-cooperative/keyring-rs)
- [oo7 crate](https://crates.io/crates/oo7)
- [linux-credentials/oo7](https://github.com/linux-credentials/oo7)
- [secrecy crate](https://crates.io/crates/secrecy)
- [iqlusioninc/crates](https://github.com/iqlusioninc/crates)
- [zeroize crate](https://crates.io/crates/zeroize)
- [russh crate](https://crates.io/crates/russh)
- [openssh crate](https://crates.io/crates/openssh)
- [openssh-mux-client crate](https://crates.io/crates/openssh-mux-client)
- [ssh-key crate](https://crates.io/crates/ssh-key)
- [RustCrypto/SSH](https://github.com/RustCrypto/SSH)
- [ssh-agent-client-rs crate](https://crates.io/crates/ssh-agent-client-rs)
- [nresare/ssh-agent-client-rs](https://github.com/nresare/ssh-agent-client-rs)
- [ringbuf crate](https://crates.io/crates/ringbuf)
- [ringbuf repo](https://github.com/agerasev/ringbuf)
- [memmap2 crate](https://crates.io/crates/memmap2)
- [memmap2 repo](https://github.com/RazrFalcon/memmap2-rs)
- [lru crate](https://crates.io/crates/lru)
- [mini-moka crate](https://crates.io/crates/mini-moka)
- [zellij-server on crates.io](https://crates.io/crates/zellij-server)
- [zellij repo](https://github.com/zellij-org/zellij)
- [par-term-emu-core-rust](https://github.com/paulrobello/par-term-emu-core-rust)
- [par-term-emu-core-rust on crates.io](https://crates.io/crates/par-term-emu-core-rust)
