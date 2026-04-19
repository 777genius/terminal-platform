# Terminal Feature Ideas Backlog

**Проверено**: 2026-04-19

Этот файл собирает не "всё интересное подряд", а только те идеи, которые реально могут усилить terminal feature в этом репо.

## North Star Principles

### 1. Transparent PTY core

Не перехватывать shell настолько глубоко, чтобы ломать TUI, keybindings и muscle memory.

Что делать вместо этого:

- держать честный PTY terminal
- добавлять shell integration поверх него
- отдельно строить command markers, search, notifications и command metadata

Источники:

- [JetBrains Terminal: A New Architecture](https://blog.jetbrains.com/idea/2025/04/jetbrains-terminal-a-new-architecture/)
- [VS Code Terminal Shell Integration](https://code.visualstudio.com/docs/terminal/shell-integration)
- [WezTerm Shell Integration](https://wezterm.org/shell-integration.html)

### 2. Session-first, not terminal-widget-first

Терминал должен быть не только view-компонентом, а session object с жизненным циклом.

### Protocol-first, FFI-second public design

Идея из Rust embed boundary research:

- if the package should work in many host apps and languages, a versioned protocol is often a healthier primary contract than direct host bindings
- FFI adapters should map onto that contract instead of becoming the only place where product semantics live

### Keep control plane and byte plane as different contracts

Идея из deeper public-protocol research:

- lifecycle commands, queries, topology, capabilities and errors are not the same class of data as PTY bytes, snapshots and replay chunks
- mixing them into one accidental transport usually makes versioning, replay and external tooling worse

### Zero-copy belongs inside the runtime, not in the public truth model

Идея из `rkyv` and `zerocopy` research:

- internal caches, snapshots and hot-path parsers may use zero-copy tools
- public compatibility should still be defined by deliberate protocol rules, not by Rust memory layout tricks

### Prefer protocol-first public transport, not shared-memory-first architecture

Идея из deeper `memmap2` / `shared_memory` / `shmem-ipc` research:

- shared memory may be a strong internal optimization or bulk-artifact seam
- but session truth, attach semantics and host compatibility should still be defined by explicit protocol contracts

### Local daemon auth should be policy-first, not path-first

Идея из deeper `interprocess` / `rustix` / `nix` / named-pipe security research:

- a local endpoint name is not enough to define trust
- attach authority should come from verified peer identity and runtime policy

### Daemon ownership should be lock-first, not pidfile-first

Идея из `fs4`, `fd-lock`, `single-instance`, `pidlock` and `pidfile-rs` research:

- one canonical runtime owner should be coordinated through explicit lock strategy
- pidfiles and singleton helpers are useful adjuncts, but too weak as the primary ownership contract

### Readiness should be declared by handshake, not inferred from socket reachability

Идея из `semver`, `backon`, `wait-timeout` and daemon-readiness research:

- host should learn phase, compatibility and capabilities from a structured handshake
- successful connect or ping alone is too weak to mean "ready"

### Use `mmap` and `memfd` for bulky immutable artifacts, not live runtime truth

Идея из deeper `memmap2` and `memfd` research:

- large snapshots, spill blobs and published immutable artifacts are a healthier fit for mapped or sealed memory
- hosts should still consume typed projections instead of raw mapped emulator state

### Public library errors should be typed and boring

Идея из `thiserror` research:

- embeddable runtime crates should expose deliberate error enums/structs instead of catch-all app-style results
- the derive tool should not leak into the compatibility contract

### Fancy diagnostics belong to app leaves, not runtime APIs

Идея из `miette` research:

- standalone CLI and desktop shells can have rich human-facing diagnostics
- runtime crates should keep their public signatures machine-friendly and host-neutral

### Prefer typed protocol and config over dynamic plugin ABI in v1

Идея из deeper extensibility research:

- most extension needs are healthier above the runtime core via config, protocol and sidecars
- in-process Rust plugin ABI should be a later product decision, not a default architectural assumption

### Separate Cargo features from runtime capabilities

Идея из deeper compatibility-matrix research:

- compile-time inclusion and runtime availability are different truths
- hosts need explicit runtime capability negotiation instead of guessing from build shape

### Separate runtime capabilities from authority

Идея из deeper compatibility-matrix research:

- a capability being compiled and available does not mean every client/session may use it
- controller/viewer roles and security policy need their own layer

### Prefer separate leaf crates over giant feature-soup core crates

Идея из deeper compatibility-matrix research:

- large optional adapters like Node bindings, C ABI, SSH and sandbox hosts are often healthier as separate crates
- this keeps core defaults smaller and semver surface clearer

### Treat compatibility matrix testing as part of the public contract

Идея из `cargo-msrv`, `cargo-hack` and `cargo-minimal-versions` research:

- MSRV, feature combinations and dependency floors should be tested intentionally
- otherwise published compatibility becomes folklore

### Document feature flags as part of the package API

Идея из `document-features` research:

- if feature flags exist, they should be visible and explained from the actual Cargo surface
- public users should not need to reverse-engineer them from source

### Keep default features minimal

Идея из deeper compatibility-matrix research:

- default builds should not silently pull in heavy remote, sandbox or adapter surfaces
- optional power should stay opt-in unless it is truly core product behavior

### Keep durable truth, append-only logs and semantic timeline separate

Идея из deeper durable-state research:

- structured truth, operational append logs and user-facing semantic timeline are different durable concepts
- collapsing them makes migrations, rebuilds and restore semantics much uglier

### Treat read models as rebuildable

Идея из deeper durable-state research:

- search indices, semantic caches and other projections should be versioned and rebuildable
- they should not quietly become the only durable source of truth

### Version snapshot formats separately from relational schema

Идея из deeper durable-state research:

- committed screen snapshots and replay blobs evolve differently than SQLite tables
- one schema version number is not enough for a serious terminal runtime

### Prefer SQLite for evolving structured truth

Идея из `rusqlite` and `rusqlite_migration` research:

- embedded relational truth with boring migrations looks like the healthiest default for sessions, topology metadata and restore bookkeeping
- specialized KV/blob stores should earn their place instead of replacing that by default

### Add KV/blob stores only when blob pressure proves it

Идея из `redb`, `heed` and `fjall` research:

- replay mirrors, spill data and heavy blobs may deserve a second store
- but dual-store architecture should be a response to real pressure, not the starting assumption

### Keep host embedding boundary separate from extension boundary

Идея из deeper Wasm/plugin sandbox research:

- host SDKs and third-party extension APIs solve different problems
- if they collapse into one boundary, plugin lifecycle and sandbox policy start polluting host ergonomics

### Prefer sidecar isolation before in-process sandbox complexity

Идея из deeper Wasm/plugin sandbox research:

- for risky or experimental enrichers, out-of-process isolation is often the cleanest first move
- it keeps fault containment and capability policy clearer than prematurely embedding a whole in-process plugin runtime

### If extension contracts harden, prefer WIT over ad-hoc plugin ABI

Идея из `wit-bindgen` and `wasmtime` research:

- typed interface contracts are healthier than callback soup or raw ABI exports
- especially if extensions may later be written in more than one language

### Treat Extism as a framework choice, not a tiny helper crate

Идея из `extism` research:

- Extism gives a serious plugin worldview with its own host/guest ergonomics
- adopting it should be a deliberate product decision, not casual dependency creep

### Keep distributed component RPC out of the v1 core

Идея из `wRPC` research:

- component-native RPC is powerful for future distributed extensions
- but it is later than a local terminal runtime needs on day one

### Never leak internal storage keys into host-facing APIs

Идея из state/handle-model research:

- internal indices, slab slots and generational keys are process-local implementation details
- public contracts should use opaque stable IDs instead

### Prefer opaque handles over raw pointers in host SDKs

Идея из deeper FFI and host-SDK boundary research:

- host languages should see stable handles and typed envelopes, not borrowed Rust internals
- this keeps object lifetime, migrations and adapter diversity manageable

### Event pumps and streams should be primary, callbacks secondary

Идея из deeper host-SDK boundary research:

- foreign callbacks are useful convenience, but they should not become the only event contract
- explicit drain/poll or framed stream semantics make multi-client attach, replay and testing much healthier

### Do not let any one binding define product semantics

Идея из `napi-rs`, `Diplomat`, `UniFFI` and `safer-ffi` research:

- Node, C ABI and generated SDKs should all map onto the same runtime truth
- adapter ergonomics may differ, but replay, overflow, close/drop and identity semantics should not

### Keep callback sugar above the runtime, not inside it

Идея из FFI boundary and runtime hot-path research:

- runtime should emit envelopes into deliberate queues or streams
- adapters may re-emit them as callbacks, promises or host-native events
- correctness must not depend on foreign callback timing

### Separate operations from subscriptions in public host SDKs

Идея из deeper async host-loop research:

- one-shot async work and long-lived terminal streams have different lifecycle and cancellation semantics
- promising both through one generic async abstraction usually creates accidental complexity

### Make cancellation explicit, not GC-driven

Идея из deeper async host-loop and FFI research:

- dropping a host object or ignoring a promise should not be the primary cancellation model
- runtime should expose explicit operation/subscription cancellation semantics

### Keep host main-thread affinity out of the core runtime

Идея из `napi-rs`, UniFFI async and multi-language adapter research:

- Node, Swift, Python and other hosts each have different event-loop or UI-thread rules
- runtime should emit ordered events, and adapters should marshal them safely onto host-specific loops

### Async FFI helpers belong only at narrow boundaries

Идея из `async-ffi` research:

- exporting FFI-safe futures can be useful at the very edge
- but core session/runtime APIs should still be modelled in terms of operations, subscriptions and typed events

### Make ownership visible in public FFI types

Идея из deeper memory-ownership research:

- borrowed inbound views and Rust-owned outbound blobs should be different carrier types
- do not rely on prose docs alone to communicate who frees what

### Destructor responsibility must stay with the allocating library

Идея из `ffi-support::ByteBuffer` and `RustBuffer` research:

- one shared library should free only the memory it allocated itself
- cross-dylib allocator assumptions are too fragile for a world-class public package

### Use more than one string carrier shape

Идея из `safer-ffi`, `Diplomat` and UniFFI memory-carrier research:

- `char *` is useful only for some C-facing edges
- general host-neutral contracts often want UTF-8 views or owned repr(C) string/blob carriers instead

### Keep payload carriers flat and self-contained

Идея из deeper FFI blob/slice research:

- snapshots, transcript chunks and search results should prefer flat blobs, slices or flat projections
- do not leak nested pointer graphs or borrowed emulator internals into host APIs

### Separate domain errors from boundary faults

Идея из deeper panic/unwind and fault-policy research:

- invalid user input, missing session and unsupported capability are not the same class of failure as panic-at-boundary or bad foreign callback
- the public contract should model them separately

### Panic text is telemetry, not public API

Идея из `ffi-support`, UniFFI callback and unwind research:

- hosts should get typed fatal categories
- panic payloads and backtraces belong in logs and diagnostics, not in the stable SDK contract

### Keep Rust as the true owner of the runtime graph

Идея из deeper object-lifetime and cycle research:

- host SDK objects should wrap runtime identities, not become the actual owners of sessions and panes
- detached sessions must remain alive or die by explicit runtime policy, not by accidental host GC

### Make subscriptions explicit owned resources

Идея из deeper lifetime and subscription-ownership research:

- screen, transcript, search and event subscriptions should have opaque IDs and explicit close semantics
- listener lifetimes should not be hidden inside callback registration folklore

### Use weak observer edges for UI and tooling watchers

Идея из deeper cycle-avoidance research:

- observers usually need visibility, not ownership
- status panels, inspectors, search watchers and UI surfaces should not keep the session graph alive by accident

### Treat stale-handle and wrong-owner as normal public outcomes

Идея из `ffi-support::HandleError` research:

- use-after-close and wrong-registry usage are routine realities in a multi-host terminal SDK
- model them deliberately instead of collapsing everything into one generic invalid-handle error

### Do not export cross-language shared ownership as the public truth model

Идея из `UniFFI`, `cxx` and deeper lifetime research:

- foreign callbacks and wrapper objects can easily create cycles or ambiguous destroy ordering
- shared ownership may exist inside one adapter, but it should not define the universal product contract

### Keep one PTY port, but separate Unix and Windows leaves

Идея из deeper PTY backend and OS seam research:

- host-neutral runtime should not see platform-specific PTY APIs
- but infrastructure should still model Unix PTY and Windows ConPTY as genuinely different leaves

### Make initial PTY dimensions first-class launch input

Идея из `portable-pty`, ConPTY and restore-order research:

- starting size affects wrapping, cursor state and terminal correctness
- late resize is not a substitute for correct initial dimensions

### Treat descriptor and handle hygiene as part of the PTY adapter contract

Идея из `portable_pty::unix` docs and platform research:

- inherited FDs and leaked handles are not incidental bugs around the adapter
- the PTY leaf should own cleanup policy for child launch boundaries

### Make Windows PTY support an explicit product policy

Идея из ConPTY research:

- supporting Windows means choosing a minimum OS baseline and explicit fallback policy
- ConPTY version gates and lifecycle quirks should not remain hidden assumptions

### Session poison should be first-class

Идея из deeper fault-containment research:

- one broken session should not automatically imply full-runtime death
- the model should allow poisoning or invalidating a single session/subscription explicitly

### Reserve abort for truly unsafe no-recovery boundaries

Идея из `abort_on_panic`, `unwind_aborts`, `nounwind` and `tarnish` research:

- abort is a real tool, but it should be a deliberate leaf policy, not the default answer to every failure
- crash containment above that should be expressed through typed fatal states or process isolation seams

### Host apps must be leaves over runtime crates

Идея из `Alacritty`, `WezTerm` и `Rio` workspace topology:

- standalone app, Electron bridge and future other hosts should depend on runtime crates instead of defining runtime truth themselves
- the more host code owns lifecycle semantics, the less reusable the package becomes

### Self-describing configs and manifests

Идея из `serde + schemars` research:

- if non-Rust hosts configure the runtime, config shapes should be schema-exportable
- generated schema helps validation, docs, host-side forms and compatibility tooling

### Keep config model independent of any one loader crate

Идея из `config-rs` and `Figment` research:

- runtime core should own config structs and validation rules
- file/env layering strategy should remain an app-layer choice

### Keep public IDs and internal IDs as separate concepts

Идея из `slotmap`, `generational-arena`, `uuid`, `ulid` research:

- a session may need a stable host-facing identity and a separate internal storage key
- conflating them makes migrations, persistence and future refactors much harder

### Use path-aware config errors

Идея из `serde_path_to_error` research:

- host integrations need exact field paths on deserialize failures
- "config invalid" is too weak for embeddable runtime UX

### Prefer generational internal registries for dynamic entities

Идея из `slotmap` and `generational-arena` research:

- stale references are not an edge case in terminal runtimes with sessions, panes and subscriptions
- generational keys are a healthier default than raw indices once references cross subsystem boundaries

### Separate command, state, event and byte lanes

Идея из deeper orchestration research:

- different traffic classes want different ordering, fanout and backpressure semantics
- one generic event bus usually degrades both correctness and operability

### Prefer owner-task truth over shared concurrent maps

Идея из `tokio`, `ractor`, `dashmap`, `scc` research:

- session truth is healthier when one runtime owner serializes mutation
- concurrent maps are stronger as secondary indexes than as primary domain ownership

### Use watch-like channels for latest state, not event replay

Идея из deeper channel-semantics research:

- current phase/health/title/cwd metadata is latest-state traffic
- it should not be modelled as "every subscriber must receive every change forever"

### Use broadcast only where all active subscribers should see the event

Идея из `tokio::broadcast` and `async-broadcast` research:

- attention events, timeline commits and viewer fanout are different from commands and state
- choose broadcast deliberately, not as a default for every cross-task message

### Keep supervision semantics explicit

Идея из `ractor` runtime semantics:

- stop, kill, restart and failure propagation are product semantics
- if an actor framework is used, its supervision model must stay a deliberate architectural choice

### Keep parser, emulator, screen model and host UI as different seams

Идея из `vte`, `vt100`, `avt`, `alacritty_terminal`, `crossterm`, `ratatui` research:

- raw escape parsing is not the same thing as terminal emulation
- headless screen snapshots/diffs are not the same thing as the main emulator port
- host-side terminal UI libraries should not become core runtime dependencies

### Treat headless terminal surfaces as first-class

Идея из `vt100`, `avt` and `shadow-terminal` research:

- snapshots, diffs, step-by-step replay and rendered-cell inspection are not just test hacks
- they are useful seams for automation, validation, restore and external tooling

### Do not let popular TUI crates leak into runtime truth

Идея из `crossterm` and `ratatui` research:

- popularity and documentation quality do not make a crate the right architectural center
- host shells can use them, but runtime truth should stay host-neutral

### Separate hot replay, durable snapshots and external projections

Идея из deeper snapshot/replay-format research:

- replay queue, persisted snapshot and tooling export have different lifecycles and constraints
- one format for everything usually creates coupling in the worst possible place

### Compression belongs after snapshot formation, not in the hot path

Идея из `zstd` and replay-buffer research:

- PTY output path should stay cheap and bounded
- durable compression should happen in a later stage with explicit policy

### Use fingerprints and checksums for different jobs

Идея из `blake3` and `crc32fast` research:

- fast corruption checks and stable snapshot identity are not the same requirement
- use a cheap checksum for corruption and a stronger fingerprint for dedupe/identity

### Treat ANSI/state-formatted output as a projection

Идея из `vt100` research:

- formatted terminal state and diffs are powerful recovery/export surfaces
- but they should stay a derived representation, not the only durable truth

### Keep binary blob encoding wrapped in versioned envelopes

Идея из `rmp-serde`, `postcard` and `bincode` research:

- raw serializer output should not become the compatibility contract by accident
- snapshot kind, version, compression and integrity metadata deserve explicit fields

### Treat restore as an ordered protocol, not a deserialize call

Идея из deeper restore/rehydrate research:

- dimensions, mode state, replay tail and reveal timing all affect correctness
- restoration needs an explicit sequence of steps, not a single "load state" operation

### Keep alternate screen as first-class restore state

Идея из `alacritty_terminal` and `vt100` research:

- alternate screen is not just extra scrollback
- it has different cursor, selection and replay semantics and should be modelled explicitly

### Resize must be restore-aware

Идея из `alacritty_terminal` grid storage research:

- buffer growth, shrink and ring-buffer layout can change recovery meaning
- resize should be treated as part of rehydrate protocol, not cosmetic post-processing

### Reveal only after state and size converge

Идея из hydration and restore research:

- early attach/reveal makes corruption user-visible and can confuse shell/TUI behavior
- hosts should project only converged state whenever possible

### Keep host input DTOs separate from terminal protocol encoding

Идея из deeper input-path research:

- UI should describe intent like key/text/paste/mouse/focus
- Rust runtime should decide how that intent becomes PTY bytes based on terminal modes

### Terminal-generated replies need their own lane

Идея из `libghostty-vt` `on_pty_write` research:

- device/status replies, OSC responses and similar output are not user input
- they should not be hidden inside generic write helpers or UI event code

### Shell integration should be an adapter, not a regex side effect

Идея из `libghostty-vt::osc`, `alacritty_terminal` and previous product research:

- cwd, semantic prompt, clipboard, title and shell compatibility belong in an explicit protocol/policy layer
- terminal packages should not smear this behavior across renderers and input widgets

### Prefer byte-first prefilters before regex captures

Идея из deeper Rust semantic-runtime research:

- hot-path analyzers should not start with "turn everything into `String` and run regex"
- `memchr`, `bstr` and multi-literal prefilters are a healthier first stage before heavier parsing

### Use multi-literal search for fixed detector families

Идея из `aho-corasick` research:

- slash commands, tool names, provider names and stable approval/error phrases are often a better fit for multi-pattern literal search than for giant alternation regexes
- this keeps hot-path candidate detection cheaper and easier to reason about

### Keep parser combinators as narrow seams

Идея из `winnow`, `nom`, `chumsky` and `logos` research:

- structured shell markers and small payload grammars can justify a parser seam
- raw PTY stream analysis usually should not be turned into a full parser-combinator pipeline

### ANSI normalization should be streaming or derived, not regex-stripped truth

Идея из `vte`, `anstyle-parse` and `strip-ansi-escapes` research:

- analyzer input should come from a streaming normalization step or from a derived emulator projection
- regex-style ANSI stripping is acceptable for export and leaf tooling, but weak as canonical runtime truth

### Command timeline should stay a derived projection

Идея из `Hermes` `NodeBuilder` and bounded analyzer research:

- completed commands, summaries and attention states are not the same thing as raw output history
- timeline entries should be derived from analyzer state, not become the primary execution truth

### Separate PTY capability from process supervision policy

Идея из deeper PTY child-lifecycle research:

- opening a PTY and supervising the child process tree are different concerns
- process groups, sessions, job objects and kill-on-drop should live in an explicit supervision layer

### Prefer composable supervision policies over one giant process abstraction

Идея из `process-wrap` research:

- one wrapper per concern scales better than a fake cross-platform "do everything" child API
- process group, session, job object and timeout escalation should remain explicit policy knobs

### Keep signal logic narrow and internal

Идея из `signal-hook` research:

- signal handling is global and easy to get wrong
- runtime architecture should keep signal coordination in narrow internal adapters instead of spreading it across host-facing APIs

### Host APIs should not expose syscall-shaped lifecycle semantics

Идея из `portable-pty`, `nix` and `rustix` research:

- hosts should ask for `terminate`, `detach`, `force_kill`, `attach`, `await_exit`
- they should not need to know about `setsid`, `killpg`, raw handles or job-object plumbing

### Local-first daemon transport should be the primary runtime boundary

Идея из deeper daemon-topology research:

- embeddable terminal runtimes benefit more from a stable local socket protocol than from making any one host binding the source of truth
- Electron, standalone desktop apps and future polyglot hosts can all adapt to that same boundary

### Keep control plane and byte plane as separate transports or lanes

Идея из `interprocess`, `bytes` and donor daemon products:

- lifecycle commands, attach negotiation and topology metadata want framed messages
- PTY bytes, replay chunks and snapshots want a separate data-oriented path

### RPC frameworks should stay outer facades

Идея из `jsonrpsee`, `tarpc` and `tonic` research:

- JSON-RPC, gRPC and Rust-first RPC tools are useful, but they should not define the core runtime truth model
- the daemon should own one local-first protocol and let higher-level APIs adapt to it

### Multi-client attach roles should be explicit

Идея из `zinc`, `gritty`, `missiond` and earlier control-surface research:

- controller/viewer or attach ownership semantics should be modelled deliberately
- reconnect, replay start and takeover policy should not be accidental behavior

### Emulator internals should not be the host render API

Идея из deeper render-model research:

- internal grid references and mutable emulator cells are too unstable to be the main UI boundary
- host UIs should consume explicit snapshots and deltas instead

### Runtime should own width, grapheme and wrap semantics

Идея из `unicode-width`, `unicode-segmentation`, `alacritty_terminal` and `libghostty-vt` research:

- cursor position, soft wrap, cell occupancy and selection semantics should be resolved before data reaches the host UI
- host renderers should not independently recompute terminal layout rules

### Ambiguous-width policy must be an explicit product choice

Идея из deeper `unicode-width` research:

- CJK-sensitive width handling changes cursor movement, wrap and selection behavior
- do not let this be an accidental compile-time or locale side effect

### Keep normalization out of live terminal truth

Идея из `unicode-normalization` research:

- normalization is useful for search, indexing and semantic matching
- but live transcript and screen truth should not be silently rewritten

### Generic line-breaking must stay separate from terminal soft wrap

Идея из `unicode-linebreak` and `textwrap` research:

- prose wrapping and terminal wrapping solve different problems
- export and adjacent surfaces may use Unicode line breaking, while screen truth still follows terminal cell semantics

### Bidi-aware presentation should be explicit, not ambient

Идея из `unicode-bidi` research:

- if bidi or advanced presentation is ever added, make it an explicit export/viewer surface
- do not silently let it rewrite core emulator cell order

### Use multiple verification layers, not one giant testing style

Идея из deeper conformance and fuzzing research:

- property tests, snapshot tests, PTY interaction tests, fuzz targets and compatibility corpora each catch different failure classes
- a terminal runtime needs several of them at once

### Keep external compatibility corpora as first-class regression inputs

Идея из `vttest` / `esctest` references and terminal emulator practice:

- do not rely only on self-invented examples
- pin external suites or mirrored fixtures and run them through a host-neutral harness

### Snapshot projections, not arbitrary internals

Идея из `insta` and projection-contract research:

- snapshot `ScreenSnapshot`, `ScreenDelta`, transcript and timeline projections
- avoid snapshotting unstable internal structs that are not part of the promised contract

### Fuzz parser, replay and protocol seams continuously

Идея из `cargo-fuzz` research:

- VT parsing, OSC parsing, framed decoding, replay merge and rehydrate code deserve dedicated fuzz targets
- these are product-quality gates, not optional extras

### Treat resize plus scrollback plus restore as named compatibility contracts

Идея из earlier runtime research plus deeper compatibility pass:

- these behaviors should have explicit non-regression coverage
- do not rely on visual smoke tests to catch them

### Keep glyph IDs and font handles out of the public terminal contract

Идея из deeper font-shaping and renderer-boundary research:

- host-neutral APIs should talk in cells, clusters, styles and projections
- glyph cache internals and font engine details belong in renderer leaves

### Font discovery should stay a renderer or app-shell concern

Идея из `fontdb` and renderer-stack research:

- system font lookup and fallback are highly host/platform specific
- the universal runtime should not force one font discovery model on all embedders

### Renderer choice must not change terminal semantics

Идея из deeper render-boundary research:

- swapping renderer stack should not change wrap, selection, cursor movement or search hit mapping
- if it does, too much truth leaked out of the core

### Optional reference renderer crates are healthy

Идея из `cosmic-text`, `swash`, `glyphon` and adjacent renderer research:

- a strong package can ship optional renderer leaves for standalone apps or reference integrations
- this is healthier than letting the core runtime absorb font shaping and glyph cache responsibilities

### Keep live diff projection and snapshot projection separate

Идея из `alacritty_terminal`, `vt100` and `shadow-terminal` research:

- live UI wants dirty regions or compact deltas
- tooling, export and reattach want full structured screen snapshots
- using one structure for both usually bloats the hot path or weakens the external contract

### Formatted export should be a separate surface

Идея из `vt100` and `libghostty-vt` formatter research:

- plain text, VT-preserving output and HTML are separate projection use cases
- they should not leak into the live render API or force the UI to rebuild them ad hoc

### Model side effects as capabilities plus typed ports

Идея из deeper capability/security research:

- file access, URL open, process launch and clipboard writes should not be ambient conveniences in the core
- they should be explicit authorities routed through named effect ports

### Prefer capability-based file authority over path-string trust

Идея из `cap-std` research:

- if the package should be reusable and trustworthy, file authority should be narrower than "whatever the process can open"
- worktree/session roots are better modelled as capabilities than as raw paths

### Secrets should be wrapped, not merely redacted in logs

Идея из `secrecy` and `zeroize` research:

- auth material, forwarded credentials and sensitive session data should use dedicated secret wrappers
- avoiding accidental log exposure is only half the job; memory lifetime matters too

### Shell splitting should stay peripheral

Идея из `shlex` and `shell-words` research:

- shell-like parsing is useful when the product explicitly accepts shell syntax
- it should not become the default spawn/config API for the runtime itself

### Paste needs an explicit safety policy

Идея из `libghostty-vt::paste` research:

- bracketed paste wrapping, control-byte handling and unsafe newline behavior deserve first-class policy
- paste should not be treated as just another text string

### 5. One typed control surface, not many accidental entry points

Desktop IPC, web shell and remote/runtime clients должны по возможности бить в один явный command/query facade, а не в набор несвязанных helper-методов.

### 3. Agent workflows need attention UX

Пользователь должен мгновенно видеть:

- где agent waiting
- где failure
- где unread event
- где long-running background task закончилась

### 4. Persistence matters more than cosmetic polish

Resume/reopen/restore/history часто ценнее, чем ещё один renderer optimisation pass.

## P0 - Must Have

### Feature slice по стандарту

Разложить новую terminal feature в:

```text
src/features/terminal/
  contracts/
  core/
    domain/
    application/
  main/
    composition/
    adapters/
      input/
      output/
    infrastructure/
  preload/
  renderer/
```

И заранее определить owner tables хотя бы для:

- terminal session durable state
- runtime observation state
- layout state
- attention/status state

### Avoid a giant generic utils crate

Идея из `Zellij` workspace topology:

- shared crates should be explicit by role like `protocol`, `config`, `telemetry-types`
- generic `utils` crates become architecture debt very quickly in long-lived terminal platforms

### Separate PTY port from emulator port

Идея из Rust stack research:

- pseudo-terminal lifecycle and terminal emulation are different seams
- keep `PtyPort` swappable independently from `EmulatorPort`
- this makes it much easier to evolve from conservative to modern emulator cores later

### Session/mux runtime deserves its own crate

Идея из `WezTerm` workspace topology:

- session orchestration, attach/detach, routing and remote-ish concerns deserve a crate boundary separate from emulator and PTY adapters
- otherwise the emulator crate tends to absorb product runtime concerns

### Prefer explicit composition roots over DI containers

Идея из deeper Rust ports/adapters research:

- wiring the runtime explicitly keeps ownership, startup policy and host-specific assembly visible
- container frameworks make it harder to keep a reusable runtime host-neutral

### Keep application services concrete and ports narrow

Идея из deeper Rust ports/adapters research:

- traits should mark real external seams, not every internal collaboration
- over-traiting the core makes ownership and async semantics blurrier

### Make async trait boundaries deliberate

Идея из `async-trait` and `trait-variant` research:

- async traits in Rust still need careful design when dyn dispatch or Send variants matter
- use helper crates at real boundaries instead of turning every trait into boxed async indirection

### Keep `tower` at outer request/response edges

Идея из `tower` and `tower-service` research:

- middleware-oriented service abstractions are great for daemon/API facades
- they should not become the universal language of the whole runtime core

### Do not let test tooling dictate production abstractions

Идея из `mockall` research:

- mocks are valuable at port boundaries
- production traits should exist because the seam is real, not because a mocking library wants a trait

### Treat downcasting and enum-dispatch as internal escape hatches

Идея из `downcast-rs` and `enum_dispatch` research:

- downcasting is for special extension islands, not routine application flow
- enum-dispatch is for closed internal families, not open reusable ports

### Keep ordered containers where order is domain truth

Идея из `WezTerm` `window.rs`:

- window and tab order are not an incidental implementation detail
- if users feel the order directly, the runtime should model that order explicitly instead of hiding it inside a generic graph abstraction

### Keep pane split topology separate from projected geometry

Идея из `WezTerm` `tab.rs`:

- split tree and positioned pane rectangles are different concepts
- live layout truth should stay small and modeful
- screen-space geometry should be derived as a projection

### Keep pane groups as a separate bounded context

Идея из `Zellij` `pane_groups.rs`:

- multi-pane selection and grouping should not become random booleans on pane state
- per-client grouping and grouped actions deserve an explicit service/store boundary

### Separate live topology from persisted layout metadata

Идея из `Zellij` `session_layout_metadata.rs`:

- restore-friendly layout metadata should be modelled separately from the hot mutable tab/session objects
- dirty-layout detection deserves its own comparison model instead of falling out of generic serialization

### Do not let `Tab` become a god aggregate

Идея из `Zellij` `tab/mod.rs`:

- once one aggregate starts owning split topology, viewport state, grouping, clipboard, layout swapping, images and interaction policy, reusable architecture starts collapsing
- prefer a smaller topology-focused tab aggregate with adjacent services and projections

### Never make raw Rust ABI the public package contract

Идея из Rust embed boundary research:

- public package compatibility should not depend on compiler-version-coupled Rust ABI details
- if this terminal package is meant for many hosts and languages, stable contracts should be protocol and/or deliberate C ABI shaped

### Code-first Rust RPC is fine internally, weak publicly

Идея из `tarpc` research:

- code-first service macros are ergonomic inside Rust
- they are a poor source of truth for a package that must be embedded from many languages and runtimes

### Prefer explicit schema evolution when the public protocol hardens

Идея из `prost` and `prost-reflect` research:

- once the package needs a stronger remote/public compatibility story, schema-first contracts are healthier than ad-hoc JSON blobs
- protobuf plus reflection is more realistic for long-lived external tooling than inventing a custom binary format too early

### Keep WIT/component-model as an extension seam, not the first host seam

Идея из `wit-bindgen` and `wasmtime` research:

- component model is promising for plugins and sandboxed extensions
- host-app embedding for a PTY-heavy runtime should not depend on a whole extra component runtime from day one

### Distinguish emulator core from terminal toolkit

Идея из WezTerm stack:

- `wezterm-term`-style emulator core and `termwiz`-style toolkit are different responsibility layers
- avoid treating parser/state core, capability helpers and higher-level widgets as one inseparable dependency choice

### Backpressure / flow control

Текущий legacy path пишет PTY output напрямую в terminal renderer. Для log-heavy команд это риск.

Нужно:

- bounded buffering
- ACK/backpressure strategy
- pause/resume producer when needed

Источник:

- [xterm.js Flow Control](https://xtermjs.org/docs/guides/flowcontrol/)

### Framed local protocol, not newline-shaped ad-hoc transport

Идея из `tokio-util::codec` и `bytes` research:

- local daemon communication should use explicit frame boundaries over `AsyncRead` and `AsyncWrite`
- `Bytes` and `BytesMut` are better hot-path building blocks than casual `Vec<u8>` concatenation and newline parsing

### Prefer pathname sockets in controlled runtime dirs over abstract namespace by default

Идея из `unix(7)` and local-daemon-auth research:

- Linux abstract sockets are convenient but are not a healthy default security boundary
- controlled runtime directories plus explicit peer verification make a better cross-platform story

### Keep Linux-only shared-memory fast lanes optional

Идея из `shmem-ipc` and `memfd` research:

- Linux memfd/eventfd/shared-ring optimizations can be excellent acceleration leaves
- they should be negotiated explicitly and should never define the universal default contract

### Searchable scrollback

Минимум:

- local find in terminal output
- next/prev match
- highlight matches

Желательно сразу проектировать search так, чтобы он поддерживал:

- incremental stepping instead of one huge blocking pass
- viewport matches as separate read model
- selected match state independent from raw query text

### Keep terminal-local find distinct from global workspace search

Идея из `OpenCove`:

- when terminal is focused, `Cmd/Ctrl+F` should open terminal-local find, not the global search surface
- local output search and workspace-wide search solve different jobs and should not fight for ownership

### Tabs + splits

Но не только "список вкладок".  
Нужно сразу закладывать:

- active state
- unread/waiting state
- split layout model
- ordered tab semantics
- zoom/floating/suppressed modes as explicit state, not ad-hoc flags

### Shell integration

Нужно собирать:

- cwd tracking
- command start/end markers
- exit status
- prompt boundaries

Практически это означает поддержку вещей вроде:

- `OSC 7`
- `OSC 133`

### Safe links

Нужно:

- file path links
- URL links
- Ctrl/Cmd + Click semantics

### Wrapped link providers, not single-line regexes

Идея из `OpenCove`:

- useful file and URL links often span wrapped lines
- link detection should tolerate line/column suffixes, wrapped URLs and context-specific false-positive filtering

## P1 - Strong Differentiators

### Session persistence

Идея из `zmx` / `Factory Floor`:

- detached session survives UI remount
- reopen session with restored scrollback/state
- background tasks continue even if pane closed

### Distinguish durable fact from runtime observation

Идея из `OpenCove`:

- restart/resume logic should read durable facts, not watcher accidents
- PTY alive/exited, attach success, late async updates and UI badges are not the same truth as resumable session intent

### Separate visible transcript from durable scrollback

Идея из `OpenCove`:

- "what is currently visible" and "what can be restored later" are different read models
- transcript mirrors can power tests/debug surfaces without becoming the owner of persistence

### Owner table for recovery truth

Идея из `OpenCove`:

- workspace, terminal, task and agent-like subsystems should have explicit ownership over different durable facts
- recovery gets brittle when one layer informally stores another layer's truth

### Persistent terminal service

Идея из `terminalcp`:

- держать terminal sessions в background service, а не внутри renderer lifecycle
- дать attach/detach semantics для UI, automation и human terminal
- разделить `screen view` и `stream/log view` как разные use cases

### Prefer transparent tmux before custom mux

Идея из `Factory Floor` и Rust stack research:

- durable sessions can start with a transparent `tmux` wrapper before we commit to building our own multiplexer
- custom mux/runtime should earn its complexity by solving problems tmux-backed persistence cannot

### Canonical mux contract above Native, tmux and Zellij

Идея из deeper multi-backend mux research:

- if we support `NativeMux`, `tmux` and `Zellij`, none of them should become the domain truth except our own canonical contract
- `NativeMux` should be the reference implementation, while `tmux` and `Zellij` stay foreign backend adapters
- hosts should only see canonical IDs, topology DTOs, capability flags and degraded-mode reasons
- backend-specific ids like tmux `%pane` / `@window` or Zellij `terminal_N` / `tab_id` should stay inside adapters
- backend negotiation should be capability-based, not name-based
- separate `raw_output_stream` from `rendered_viewport_stream` in capability vocabulary
- treat backend refs as session-scoped helpers, not durable public identity
- for tmux, tab bindings should preserve session context because windows may be linked into multiple sessions
- for tmux, resize authority should be an explicit adapter policy, not an accidental byproduct of attach mode
- for Zellij, preserve typed pane refs like `terminal_N` vs `plugin_N` and keep `tab_id` separate from visual position

### Split native backends from foreign mux backends

Идея из deeper multi-backend mux research:

- native backend owns PTY truth, emulator truth and screen diff truth
- foreign mux backends own their own screen/session truth and should be imported through snapshots, subscriptions and mapping tables
- this split avoids pretending that imported `tmux` or `Zellij` sessions are the same thing as our native runtime sessions

### Serialize Zellij mutations inside the adapter

Идея из deeper Zellij programmatic-control review:

- `zellij action` is subprocess-based transport, not an ordered in-process API
- concurrent external mutations have no ordering guarantee
- `ZellijAdapter` should own a mutation lane per imported session instead of letting arbitrary callers spawn actions directly

### Prove projections before foreign backend parity

Идея из deeper multi-backend mux review:

- before building full `TmuxAdapter` or `ZellijAdapter`, freeze and test canonical `TopologySnapshot`, `ScreenSnapshot`, degraded-mode envelopes and projection-source semantics
- otherwise richer foreign backend APIs will silently shape the public contract

### Session route as part of identity

Идея из `OpenCove`:

- if a session can be local or remote, the route should be explicit state
- local session id and remote session binding are different truths and should not be collapsed casually

### Explicit write queues

Идея из `terminalcp` internals:

- отдельно сериализовать writes в PTY и writes в virtual terminal state
- не надеяться, что async callbacks сами сохранят корректный порядок
- это особенно важно для interactive TUIs и high-frequency output

### Graceful shutdown should be a first-class runtime design concern

Идея из `tokio-util` `CancellationToken` и `TaskTracker`:

- session shutdown should have explicit cancellation tree and tracked task draining
- do not rely on dropping random join handles or process objects and hoping the runtime unwinds cleanly

### Process supervision deserves explicit libraries, not spawn hacks

Идея из `process-wrap` and `signal-hook` research:

- process groups, sessions, job objects, signal masks and kill-on-drop are separate lifecycle concerns
- runtime code should model them explicitly instead of accumulating platform-specific `pre_exec` and kill glue

### Prefer `process-wrap` over deprecated `command-group`

Идея из current crate state:

- `command-group` is explicitly superseded
- `process-wrap` fits ports/adapters better because wrappers compose by single concern and by platform

### Make telemetry export optional, not the core contract

Идея из `OpenTelemetry` research:

- runtime should emit `tracing` events/spans as its native observability surface
- OTEL exporters and bridges should remain optional host-side infrastructure

### Separate PTY writer loop with hard caps

Идея из `zellij-server`:

- writes deserve a dedicated queueing loop with pending byte limits and terminal-local buffers
- if write backpressure is unbounded, interactive sessions and crash isolation both get worse

### Shared foundation resource session

Идея из `restty`:

- heavy foundation resources like WASM module, GPU core and font stores can be shared across multiple panes
- this session is not the same thing as a PTY/process session

### Output batching with idle/max thresholds

Идея из `restty`:

- PTY output buffering should have both idle flush and max flush deadlines
- this reduces renderer thrash without making continuous output feel laggy

### Terminal-generated response channel

Идея из `wterm`:

- terminal engine replies to control queries should have an explicit channel separate from user/app input
- this keeps PTY protocol semantics cleaner than pretending every outbound byte is the same class of event

### Keep emulator callbacks tiny on the hot path

Идея из `libghostty-vt`:

- effects and terminal-generated responses often run synchronously during VT processing
- expensive analytics, persistence or orchestration work should be converted into lightweight events and leave the hot path immediately

### Session phase state machine

Идея из `Hermes IDE`:

- session should expose phase like `idle / shell_ready / busy / waiting`
- smart UX should depend on phase, not only on "pane is focused"
- overlays, command helpers and suggestions should be gated by explicit runtime state

### Stable prompt state separate from transient echo state

Идея из `Hermes IDE`:

- suggestion eligibility may need a `lastStablePhase`-style signal instead of raw live phase
- shell echo or transient busy flicker should not constantly tear down smart prompt UX

### Terminal pool above UI components

Идея из `Hermes IDE`:

- не привязывать terminal instance напрямую к lifecycle React component
- держать terminal pool/module-level registry отдельно от конкретных panes
- attach/detach renderer surface to session, instead of re-creating terminal runtime on every remount

### Overlay must respect alternate screen

Идея из `Hermes IDE` и вообще modern TUI reality:

- any smart overlay must dismiss when terminal enters alt-screen
- never let suggestion UI intercept keys while `vim`, `less`, agent TUIs or other full-screen apps are active

### Foreground-process gating for prompt UX

Идея из `Hermes IDE`:

- prompt suggestions should only appear when the shell actually owns the foreground process group
- cursor position and input buffer are not trustworthy if another program owns the terminal

### Hidden textarea or equivalent IME-safe input bridge

Идея из `wterm`:

- terminal input layer should explicitly handle composition events, paste, accessibility and browser focus quirks
- "keydown on div" usually stops being enough once IME/mobile/accessibility matter

### Shared engine bootstrap

Идея из `ghostty-web`:

- if foundation engine is heavy, initialize it once per process
- terminal instances should reuse a shared engine/runtime where possible

### Stale connection guard

Идея из `restty`:

- WebSocket or remote PTY reconnect flow should carry a monotonic `connect generation` token
- stale socket callbacks must be ignored instead of racing against the current session

### Attention system

Идея из `cmux`:

- waiting/input-needed state
- failure state
- unread event state
- visible badge/ring/highlight on session/pane/tab

### Controller/viewer attach roles

Идея из `OpenCove`:

- multi-client terminal attach should distinguish who currently controls input
- viewer/controller is a useful product semantic, not just a transport detail

### Status bridge for tools and agents

Идея из `cmux`:

- host should expose simple `notify`, `set-status`, `clear-status` style primitives
- tools/hooks running inside the terminal should have a lightweight way to report state to the app shell

### Shell-aware conflict policy for smart suggestions

Идея из `Hermes IDE`:

- ghost text and Tab interception should respect shell type, native autosuggestions and whether shell integration disabled conflicts
- smart prompt UX needs explicit augment/replace/off semantics

### Session auto-discovery

Идея из `Nezha`:

- обнаружение уже запущенных relevant sessions
- восстановление связи с ними

### Session previews / dashboard mode

Идея из `webterm`:

- live previews of sessions
- fast jump from overview to active terminal
- monitoring many sessions without opening every one

### Task-oriented session API

Идея из `zmx`:

- кроме attach нужны команды уровня `run`, `wait`, `tail`, `history`
- session API может обслуживать и UI, и automation, и future agents
- это полезнее, чем думать только категориями "вот terminal pane"

### Sequence-based PTY replay

Идея из `OpenCove`:

- PTY streaming should prefer `seq + replay window + truncated flag`
- reconnect should resume from last acknowledged sequence instead of replaying a blind full snapshot

### Overflow should trigger explicit resync

Идея из `OpenCove`:

- replay gap should not silently degrade into corrupted continuity
- overflow should have its own explicit protocol event
- session can recover via fresh snapshot or explicit reattach flow

### Workspace-aware terminal metadata

Показывать рядом с terminal:

- branch
- cwd
- project
- possibly port
- last important event

### Runtime mode as explicit workspace property

Идея из `Mux`:

- local, worktree and remote runtime should be modeled as distinct product modes
- each mode should carry explicit semantics: isolation, security, filesystem path and review workflow

### Execution context should travel with session launch

Идея из `OpenCove`:

- `sessionId` alone is too weak for restore, review and remote routing
- launch results should carry project/space/endpoint/target/scope context as first-class data

### Keep runtime interface low-level

Идея из `Mux`:

- runtime abstraction should expose streaming/process/workspace primitives, not every high-level product workflow
- shared helpers and use cases can live above the runtime boundary

### Negotiate terminal/runtime capabilities explicitly

Идея из `OpenCove`:

- web and remote clients should ask for protocol version, replay limits, roles and feature flags
- avoid inferring server behavior from app version strings or optional endpoints

### Separate workspace create from runtime init

Идея из `Mux`:

- creating workspace identity and provisioning runtime environment are different phases
- useful for retries, progress reporting and restore flows

### Workstream runtime object

Идея из `Factory Floor`:

- думать не только "создать terminal session", а "создать workstream"
- workstream связывает worktree, terminal, agent, optional browser/editor, scripts and env
- deterministic env vars and ports can reduce chaos in parallel workflows

### Deterministic surface/session identities

Идея из `Factory Floor`:

- session/surface identifiers should derive from stable workspace/workstream identity and role
- this simplifies restore, reconnect, telemetry and cross-surface linking

### Workspace env contract

Идея из `Factory Floor` и `zmx`:

- terminal runtime should inject explicit env vars describing session/workspace identity
- useful for prompts, logs, scripts, debugging and agent workflows

### Compatibility env aliases for adjacent tool ecosystems

Идея из `Factory Floor`:

- workstream shell can expose compatibility env vars for neighboring tools and script conventions
- useful when adopting external worktree/agent/run-script ecosystems without forcing users to rewrite everything

### Transparent tmux persistence wrapper

Идея из `Factory Floor`:

- a dedicated-socket tmux wrapper can provide durable sessions without adding extra UI chrome
- deterministic session naming and invisible tmux config are part of the product contract, not just implementation detail

### Execution timeline

Идея из `Hermes IDE`:

- список последних команд с duration и exit code
- быстрый jump из timeline обратно в нужную session/block
- отдельная history surface рядом со scrollback, а не вместо него

### Merge shell history with session history

Идея из `Hermes IDE`:

- history-backed suggestions are stronger when they blend long-term shell history with local session execution history
- keep both recency and frequency signals

### Output analyzer seam

Идея из `Hermes IDE`:

- raw PTY output and semantic interpretation should be different layers
- provider-specific analyzers can drive phase transitions, prompt detection and future agent-aware UX

### Keep semantic runtime bounded and benchmarked

Идея из `Hermes IDE`:

- analyzer-side caches should use explicit caps and eviction
- if semantic parsing sits on the PTY hot path, it deserves real benchmark coverage instead of wishful thinking

### Benchmark and fuzz protocol hot paths as product policy

Идея из `criterion` and `cargo-fuzz` research:

- framing, replay merge, escape-sequence handling and parsers should have dedicated fuzz targets
- PTY write path, batching and snapshot generation deserve regression benchmarks instead of occasional manual checks

### Provider registry for semantic parsing

Идея из `Hermes IDE`:

- provider-specific prompt, tool and token parsing should live behind explicit adapters
- fallback generic parsing is still useful, but it should not become a giant unstructured regex pile

### Semantic session cache above raw output

Идея из `Hermes IDE`:

- keep cwd, provider, recent commands, tool summary, files touched and phase hints in analyzer/runtime state
- do not force renderer remounts or UI-only state to become the owner of semantic session truth

### Keep identity context separate from ephemeral execution context

Идея из `Hermes IDE`:

- files touched, recent errors, temporary resolutions and similar execution exhaust should not automatically become context-injection truth
- context versioning gets noisy fast when ephemeral runtime data leaks into identity state

### Visible launch failure state

Идея из `Factory Floor`:

- terminal launch failures should become explicit UI state with retry and diagnostics
- avoid silent blank panes and silent fallbacks

### Defer destructive redraw until meaningful follow-up output

Идея из `OpenCove`:

- control-only redraw chunks should not immediately replace a recovered or placeholder baseline
- wait for printable follow-up content or an explicit terminal event that justifies replacement

### Overlap-aware replay during hydration

Идея из `OpenCove`:

- when replaying live PTY continuation after restore, remove suffix/prefix overlap with the durable baseline
- duplicate output after reconnect is usually an ordering problem, not only a buffering problem

### Automatic terminal queries bypass hydration deferral

Идея из `OpenCove`:

- terminal-generated protocol traffic should not be trapped behind placeholder or redraw deferral
- delayed replies can change downstream CLI behavior, not just visual timing

### Flush queued writes before hydration replay

Идея из `OpenCove`:

- before replaying buffered hydration output, flush pending PTY write queue first
- otherwise replay can interleave with older writes and corrupt continuity

### Viewport-aware output scheduling

Идея из `OpenCove`:

- output scheduler should lower write budget while the user is actively scrolling or inspecting viewport history
- continuous output should stay live, but renderer churn must drop during viewport interaction

### Track direct-write commit state explicitly

Идея из `OpenCove`:

- do not treat `terminal.write(...)` as committed until its callback fires
- later chunks may need to queue behind that in-flight write to preserve ordering

### Serialize restore snapshot before resize

Идея из `zmx`:

- on reattach, terminal restore snapshot may need to be captured before resize/reflow
- resize can move cursor and destroy the exact frame the user expects to resume

### Initial PTY dimensions must match initial terminal buffer dimensions

Идея из `Hermes IDE`:

- start the renderer buffer at approximately the same rows/cols passed to PTY creation
- later resize cannot fully repair early wrap and cursor corruption if PTY and renderer started with mismatched widths

### Focused-terminal-only foreground polling

Идея из `Hermes IDE`:

- foreground-process checks are valuable for prompt UX, but should usually run only for the focused session
- this keeps the signal strong without turning it into noisy background load

### Re-send resize on shell-ready transition

Идея из `Hermes IDE`:

- a first resize sent during attach can be lost before the shell installs its `SIGWINCH` handling
- sending resize again when the shell is known-ready is a cheap correctness win

### Shell integration must have conflict policy

Идея из `Hermes IDE`:

- shell integration should explicitly handle conflicting autosuggestion and completion plugins
- loading user config first and then applying terminal-specific overrides is usually safer than trying to replace the whole startup path

### Clean copied terminal selection semantically

Идея из `Hermes IDE`:

- copied text should join soft wraps and some program-generated wraps using buffer metadata and heuristics
- plain DOM selection text is often not the logical command/output the user expects

### Small automation protocol over sessions

Идея из `termscope`:

- a JSON-lines session/query/snapshot protocol is enough to unlock tests, agents and CI
- useful future seam even if not exposed in the first user-facing release

### Publish bulky snapshots as artifacts, not as giant event floods

Идея из deeper `memmap2` / `memfd` research:

- if very large snapshots or replay blobs must cross a local boundary, artifact publication plus typed control metadata is healthier than forcing everything through one hot event stream

### Separate command/query control plane from sync event plane

Идея из `OpenCove`:

- request/response operations and reactive resync events should not be forced through the same channel
- state update streams deserve their own narrow protocol

### Verify peer identity explicitly on accept

Идея из `rustix`, `nix`, `unix-cred` and local-daemon-auth research:

- local sockets should not rely only on path or namespace secrecy
- Unix peer credentials and Windows pipe auth/session policy deserve explicit adapter-level checks

### Retry helpers should wait for explicit readiness, not define it

Идея из `backon`, `backoff` and `tokio-retry` research:

- reconnect and startup retries are useful tactics
- but the real contract must come from declared daemon phases and compatibility rules

### Enforce public API discipline in CI

Идея из `cargo-semver-checks` and `cargo-public-api` research:

- reusable runtime crates should not rely on human review alone to preserve compatibility
- semver linting and public API snapshots catch different failure modes and work well together

### Supply-chain policy is part of release quality

Идея из `cargo-deny`, `cargo-audit`, `cargo-vet` research:

- advisories, licenses, duplicate deps, trusted sources and audit provenance are different concerns
- serious external distribution should gate on them explicitly

### Keep Windows named-pipe ACL and impersonation policy explicit

Идея из Microsoft named-pipe security docs:

- Windows local daemon security should not rely on default pipe descriptors
- ACLs, logon/session scoping and impersonation assumptions belong in a deliberate adapter policy

### Release engineering is architecture, not ops glue

Идея из `cargo-dist`, `cross`, `cargo-zigbuild` research:

- once the project ships binaries, adapters and maybe C artifacts, artifact planning becomes part of the design
- portability testing and artifact portability tuning are different jobs and may need different tools

### Prefer explicit UTF-8 path policy at host-facing boundaries

Идея из `camino` research:

- manifests, configs, CLI args and Electron bridges get simpler if host-facing paths are deliberately UTF-8
- raw `Path` can remain inside lower OS-facing seams where needed

### Hot reload should swap immutable snapshots

Идея из `arc-swap` research:

- read-mostly runtime policies and config views are healthier as atomically replaced snapshots
- avoid mutating globally shared live config structures in place

### Caches accelerate read models, they do not own truth

Идея из `moka` and earlier recovery-truth research:

- search indexes, previews and snippets may be cached
- session truth, persistence truth and replay truth should survive cache invalidation without semantic loss

### Keep file watching optional and outside correctness-critical flows

Идея из `notify` research:

- watch mode is useful for operations and development UX
- core runtime correctness should not depend on filesystem events arriving perfectly on every platform

### Compile-time registries are useful for built-ins, not enough for external plugins

Идея из `inventory` research:

- distributed registration is great inside a linked workspace
- it should not be mistaken for a stable third-party plugin ecosystem

### Keep small-string and small-vec optimizations internal

Идея из `compact_str` and `smallvec` research:

- these are useful hot-path tools for internal labels and tiny collections
- host-facing contracts should keep using boring stable types like `String` and `Vec`

### Handshake-required streaming protocol

Идея из `OpenCove`:

- PTY streaming over WebSocket should require explicit protocol negotiation, not implicit best-effort JSON
- subprotocol, hello and protocol version checks make reconnect and compatibility rules much clearer

### Viewer/controller attach roles

Идея из `OpenCove`:

- read-only attachment and write authority should be explicit session roles
- control handoff should be a product-level event, not a side effect hidden inside a transport adapter

### Dual read models for terminal output

Идея из `terminalcp`:

- rendered screen snapshot и raw stream chunk должны быть разными contracts
- UI search/preview часто хочет одно
- logs, automation, parsing and export хотят другое

### Durable scrollback mirror

Идея из `OpenCove`:

- durable scrollback persistence should be a separate mirror adapter, not part of the live PTY hot path
- mirror can run on a timer and flush on rebinding/dispose

### Overflow should advertise snapshot recovery explicitly

Идея из `OpenCove`:

- replay overflow should not look like silent data loss
- stream protocol should explicitly tell the client when it must switch from incremental replay to snapshot resync

### Slow subscribers may need forced backpressure disconnect

Идея из `OpenCove`:

- some unhealthy websocket subscribers are safer to close than to keep buffering forever
- backpressure policy may need a hard ceiling, not only softer batching heuristics

### Fingerprint-based snapshot dedupe

Идея из `OpenCove`:

- scrollback persistence can dedupe snapshots cheaply with `length + tail`
- full-string compare is often unnecessary and wastes work on long-running sessions

### Keep remote binding identity separate from local session identity

Идея из `OpenCove`:

- a home session id and the remote/runtime session it proxies are different truths
- route/binding metadata should not be flattened into one opaque session identifier

### Browser/editor adjacency

Идея из `Factory Floor` и `cmux`:

- browser split next to terminal
- quick jump from terminal to related browser/editor/task surface

### Workstream shell owns adjacent surface lifecycle

Идея из `Factory Floor`:

- terminal, browser and editor tabs should be restored and reconciled by a workspace/workstream shell
- dead terminal surfaces should not casually destroy the rest of the adjacent workspace state

### Status bridge separate from notifications

Идея из `cmux`:

- notifications announce events, but status bridge represents current long-lived state
- states like `running`, `waiting`, `error` and `idle` deserve a separate integration surface from toast-like alerts

### Workspace-scoped browser storage and proxy identity

Идея из `cmux`:

- browser adjacency should follow workspace identity for cookies, storage and remote proxy binding
- a browser surface next to terminal is much more useful when it can survive reconnect and stay tied to workspace route/topology

### Approved adjacent surfaces instead of IDE sprawl

Идея из `OpenCove`:

- file/browser/task surfaces next to terminal should be bounded and explicit
- avoid letting random UI state become a hidden owner of terminal truth

### Snapshot-aware merge for persisted state

Идея из `OpenCove` tests:

- some persisted terminal/workspace fields may need three-way merge semantics using a base snapshot
- naive last-write-wins can clobber durable links or layout changes

### Remote runtime must be transport-scoped

Идея из `cmux`:

- if we ever add remote/sandbox terminals, proxying and reconnect should live in a transport/runtime layer
- do not bake remote semantics into pane UI logic

### Detached runtime should still answer terminal queries

Идея из `zmx`:

- when no UI client is attached, runtime may still need to respond to DA-style terminal queries
- detached mode should preserve shell expectations, not just keep the process alive

### Optional remote/sandbox backend

Идея из `Open Terminal`:

- local backend mode
- remote backend mode
- sandboxed execution option

### Threat model for remote runtime

Идея из `Mux`:

- if remote runtime appears, document what data is synced, what credentials are exposed and what isolation is expected
- security semantics must be a product contract, not an implementation afterthought

### Keep remote daemon off the hot path

Идея из `cmux`:

- remote daemon should coordinate attach/proxy/status/reconnect, but per-keystroke terminal hot path should stay lean
- do not route more through the remote coordinator than necessary

### Remote coordinator should dispatch, not become the runtime

Идея из `Factory Floor`:

- if we ever add remote dashboards or job boards, they should enqueue work and receive status instead of owning terminal runtime directly
- this keeps local execution semantics clearer and avoids turning a coordinator into a keystroke-path dependency

### Explicitly separate layout restore from process restore

Идея из `cmux` current limitations:

- layout restore сам по себе не равен durable process persistence
- если обещаем persistence, надо точно определить, что живёт после restart:
  - только layout and metadata
  - scrollback
  - or real live process state

### Preserve committed screen state for alternate buffers

Идея из `OpenCove`:

- full-screen TUI restore may require a committed serialized screen snapshot, not just raw PTY tail bytes
- replaying raw delta on top of alt-screen state can destroy the last user-visible frame

### Shell-visible session identity

Идея из `zmx`:

- session name or id should be injectable into shell env
- useful for prompt, logs, debugging and remote workflows

### Phase-gated app writes

Идея из `Hermes IDE`:

- context nudges, quick actions and automation writes should be deferred if the session is busy
- app-originated writes are safer when gated by `NeedsInput` or another explicit phase

### Backend-authoritative context orchestration

Идея из `Hermes IDE`:

- agent launch, context apply, prompt detection and deferred nudges should be orchestrated in the runtime/backend layer
- frontend can project and request, but should not own the timing-critical orchestration truth

### Prefer launch-time context seams over late PTY injection

Идея из `Hermes IDE`:

- if a provider supports a reliable launch-time prompt/context seam, use it
- PTY injection should stay a fallback, not the first-choice path

### Transport-aware context policy

Идея из `Hermes IDE`:

- local, remote and SSH-like runtimes may need different context delivery strategies
- do not pretend all sessions can consume the same local-path-based context contract

### Project cheap phase updates separately from heavy metrics updates

Идея из `Hermes IDE`:

- phase transitions often need immediate projection
- rich metrics snapshots can be throttled without hurting correctness

### Separate raw execution log from structured execution timeline

Идея из `Hermes IDE`:

- append-only event logs and human-meaningful command timeline entries serve different jobs
- timeline summaries should be modeled and persisted separately from low-level execution records

### Learn command predictions from completed executions

Идея из `Hermes IDE`:

- command suggestion memory is more robust when learned from completed command nodes than from raw keystrokes
- semantic completion data is usually less noisy than prompt-time input echo

### Keep context snapshots versioned and bounded

Идея из `Hermes IDE`:

- semantic session context should live in a versioned bounded store
- this is healthier than treating context as an unbounded append-only side effect of raw terminal output

### Foundation boundary should stay extensible

Идея из `restty`:

- interceptors, lifecycle hooks and render hooks are easier to add when the runtime seam is explicit from the start
- even if we do not expose third-party plugins, internal extension seams are still valuable

### Validate local session/socket identities strictly

Идея из `zmx`:

- session names and socket paths should be validated explicitly
- local IPC edges are easy to break with stale sockets, invalid names and path-length limits

### Per-session daemon as blast-radius limiter

Идея из `zmx`:

- durable sessions do not have to share one giant multiplexed daemon
- one daemon per session simplifies IPC, cleanup and failure isolation

### Unify external entry points through one control surface

Идея из `OpenCove`:

- desktop IPC, CLI, web and remote worker should hit the same command/query/event facade
- avoid duplicating terminal business rules across transports

### Separate topology metadata from secrets

Идея из `OpenCove`:

- remote endpoints, mounts and topology belong in durable metadata stores
- tokens/credentials should be stored separately with tighter handling

### Resize churn must preserve scrollback

Идея из `cmux` regression coverage:

- treat resize + preserved history as an explicit non-regression contract
- repeated split resizes should not silently drop older scrollback

## P2 - Advanced / Experimental

### Spatial terminal workspace

Идея из `OpenCove`:

- terminals, notes, tasks, agents on one canvas
- persistent layout as working memory

### Headless terminal inspection API

Идея из `termscope`:

- snapshot output in structured form
- drive terminal programmatically for tests
- JSON-lines or internal automation API

### Use multiple verification styles, not only unit tests

Идея из `loom`, `proptest`, `insta`, `nextest`:

- concurrency-sensitive runtime code deserves model checking
- protocol and parser logic deserves property tests
- restore and projection output deserves snapshots
- the workspace deserves a fast dedicated test runner in CI

### Choose durable store by truth shape, not by hype

Идея из `rusqlite`, `redb`, `heed`, `sled` research:

- relational metadata, indexing and restore rules often fit SQLite better
- append-ish mirrors and simple KV truth may fit `redb` better
- do not pick an embedded store only because it looks fast or modern

### Reuse the same runtime core for automation and product

Идея из `termwright`, `expectrl` и `shadow-terminal`:

- automation should hit the same session/runtime truths as the product, not a fake parallel stack
- screen snapshots, waits and input simulation are stronger when they are thin layers over the real runtime

### A local daemon can be a valid embed boundary

Идея из `termwright`:

- a narrow local socket protocol with versioned requests can be a healthier embed seam than a large direct FFI surface
- this is especially attractive when Rust owns sessions and Electron only owns product UI

### Prefer local sockets as the default daemon transport

Идея из `interprocess` research:

- cross-platform local sockets are a strong default for host-to-runtime communication
- they keep desktop embedding lean without dragging network-heavy RPC semantics into the hot path

### Keep plugin and adapter crates as leaves

Идея из `Zellij` plugin topology:

- plugins, host adapters and optional surfaces should depend on core/runtime crates
- core/runtime crates should not depend on them back

### Libraries emit tracing, hosts install subscribers

Идея из `tracing`:

- runtime crates should emit structured spans and events but must not install global subscribers
- standalone app, Electron host and tests can each choose their own collection and export policy

### C ABI adapter should be generated and packaged like a real library

Идея из `rustls-ffi`, `cbindgen`, `safer-ffi`:

- if we expose a C ABI, treat it as a serious product surface with headers, install story and `pkg-config` metadata
- ad-hoc handwritten headers and undocumented link flags do not scale

### Host-specific adapters must stay thin

Идея из `napi-rs`, `UniFFI`, `Diplomat`, `Interoptopus`:

- Node, mobile and language-specific bindings should translate host idioms onto the core contract
- they should not own session rules, replay semantics or become the only canonical API

### Structured terminal parser sidecar

Идея из `ghostty-opentui` и `ht`:

- parse ANSI to structured JSON
- generate plain text without regex hacks
- reuse parser sidecar for search/indexing/AI

### Keep foundation/app-shell boundary explicit

Идея из `wterm`, `restty`, `ghostty-web`, `libghostty`:

- terminal foundation should not secretly own product concerns
- persistence, tabs, workstreams, notifications, browser adjacency should live above core emulator layer

### Optional alternative PTY bridge

Идея из `Obsidian Ghostty Terminal`:

- sidecar PTY proxy path как fallback к `node-pty`
- useful if native addon packaging becomes painful in some environments
- особенно интересно для plugin-like or constrained Electron surfaces

### Command blocks / block timeline

Нужно не ломая PTY сделать:

- clear command boundaries
- foldable command/result groups
- jump between commands

### Notification sequences as integration surface

Идея из `cmux`:

- воспринимать `OSC 9/99/777` и похожие сигналы
- routing into app-level attention system

### Smart prompt / structured OSC UX

Идея из `termprompt`:

- terminal apps can emit structured UI hints
- terminal feature can render richer native UI while preserving fallback TUI

### Explicit remote runtime route

Идея из `wezterm-ssh`, `OpenCove`, `Mux`:

- `local`, `remote-ssh`, `remote-daemon`, `sandbox` should be explicit runtime routes
- remote should not be a hidden implementation detail inside session truth

### SSH adapter island

Идея из `wezterm-ssh`:

- keep `ssh2` or `libssh-rs` behind a narrow adapter seam
- expose same session/runtime contract to hosts for local and remote paths
- do not leak backend-specific channel handles

### System OpenSSH as outer route, not core truth

Идея из `openssh`:

- pragmatic bridge for exec/subsystem/admin flows
- reuse user SSH config and multiplexing
- do not let system `ssh` process semantics become the core remote session model

### Pure-Rust SSH only when remote becomes first-class

Идея из `russh`:

- full protocol ownership is powerful
- but should be a deliberate product bet, not a casual v1 dependency

### Append-first transcript truth

Идея из transcript/search layering:

- keep transcript truth append-first and host-neutral
- do not let rope or search index become the primary runtime source of truth

### Rope snapshots as derived read models

Идея из `crop` and `ropey`:

- use cheap rope snapshots or mirrors for background search/export/projection
- keep them separate from live PTY truth

### Separate local find from durable global history search

Идея из terminal/read-model layering:

- live scrollback find and persisted history search have different constraints
- do not force them into one storage layer

### Link extraction should be a first-class surface

Идея из `linkify`:

- URL and email boundary detection deserves its own subsystem
- better than regex piles in host UI

### Full-text engine only as a derived terminal-history surface

Идея из `tantivy`:

- great for persisted history and cross-session search
- bad as hot-path live transcript truth

### Split control plane from hot data plane

Идея из host wire-contract layering:

- control commands and session events should not share one universal payload format with screen deltas
- screen/replay/snapshot lanes deserve explicit binary chunk transport

### Public host contract should not equal durable export format

Идея из protocol/data-plane separation:

- live transport and export/debug formats can differ
- do not force one serialization framework to own every surface

### Protobuf is for hardened public envelopes, not for every byte

Идея из `prost`:

- good for stable public schemas
- not a replacement for explicit binary screen/replay lanes

### Zero-copy schema frameworks are optional future tools

Идея из `flatbuffers` and `capnp`:

- powerful for committed cross-language contracts
- too heavy to become the architectural center by default

### Explicit per-session memory budgets

Идея из backpressure/runtime layering:

- every session/workstream should have an explicit output and replay budget
- memory growth must be policy-driven, not incidental

### Hot replay tail separate from durable history

Идея из `ringbuf` and spill policy:

- small fast replay buffer for recent attach/catchup
- durable mirror for long-lived retention

### Allocation-reusing hot lanes only where proven

Идея из `thingbuf`:

- use allocation-reusing channels in specific hot lanes
- do not rebuild the whole runtime around a narrow optimization

### Rate limiting is not terminal backpressure

Идея из `governor`:

- useful for side APIs
- not the primary answer to PTY output pressure

### Resource governance must stay separate from backpressure and process supervision

Идея из `tokio`, `process-wrap`, `rlimit`, `cgroups-rs` and `systemd-run`:

- backpressure answers output and retention pressure
- process supervision answers child lifecycle
- resource governance answers budgets, deadlines and isolation policy
- mixing all three into one abstraction makes the runtime harder to reason about

### Emulator-visible scrollback policy and runtime spill policy must cooperate

Идея из `alacritty_terminal` grid/history semantics:

- history policy affects resize, display offset and visible content
- cannot be treated as a pure outer cache concern

### Public policy should express intent, not raw OS constants

Идея из `rlimit` and outer isolation leaves:

- host-facing policy should say things like wall-clock budget, concurrency budget, output budget or route budget
- raw `RLIMIT_*`, cgroup names and systemd unit details should stay in platform adapters

### Outer deployment isolation should stay optional

Идея из `cgroups-rs` and `systemd-run`:

- Linux/systemd enforcement is useful
- but embeddable terminal package must not assume these facilities exist
- deployment governance and runtime governance are different layers

### One authority should own state-directory layout

Идея из `directories` and `camino`:

- config, cache, spill, socket and durable truth paths should be derived from one central policy
- individual subsystems should not invent roots ad hoc

### Atomic publish should be a first-class port

Идея из `tempfile` and `atomic-write-file`:

- writing bytes and publishing an artifact should be separate concepts
- staged write, flush and final publish deserve reusable policy instead of open-coded file writes

### Durable truth and runtime artifacts must be different categories

Идея из `interprocess`, `rusqlite` and restart hygiene:

- sockets, pid files and transient endpoints are rebuildable runtime artifacts
- databases, manifests and committed snapshots are durable truth artifacts
- mixing their semantics makes restart logic brittle

### Lock ownership should be explicit and centralized

Идея из `fs4` and `fd-lock`:

- singleton daemon ownership, migration gates and per-workspace coordination need one coherent lock strategy
- random lock files across modules are architectural debt

### Validate before reclaiming stale runtime artifacts

Идея из daemon-ownership research:

- socket existence, pid metadata and old lock records are only hints
- contenders should validate published runtime state and attempt contact before reclaiming and rebuilding

### Restart should be modeled as recovery protocol

Идея из crash-consistency layer:

- startup should validate roots, acquire locks, open truth, rebuild ephemeral artifacts and only then expose routes
- best-effort cleanup is not enough for a reusable terminal runtime

### Publish tiny runtime ownership records, not giant mutable daemon state

Идея из daemon-ownership research:

- ownership record should help validate the current daemon instance
- it should contain instance/scope/endpoint metadata, not try to replace live protocol validation

### Keep upgrade and self-reexec as outer operational leaves

Идея из `self-replace` research:

- upgrade choreography can be valuable later for standalone distribution
- but it should not become the center of readiness or compatibility architecture in v1

### Keep `tracing` as the native semantic diagnostics surface

Идея из `tracing` ecosystem:

- runtime should speak in spans and events first
- subscribers, appenders and exporters belong to composition roots and host apps

### Metrics should stay derived and optional

Идея из `metrics` and `metrics-util`:

- counters and histograms are useful for operational surfaces
- but they should be computed from runtime truth, not replace it

### Profiling must stay opt-in and leaf-scoped

Идея из `pprof` and `console-subscriber`:

- flamegraphs, Tokio task inspection and deep runtime profiling are valuable
- but they should live in debug/admin/profile modes, not shape the minimal core

### Pretty diagnostics belong to leaf executables

Идея из `tracing-error` and `color-eyre`:

- rich panic/error reports are good for CLI, daemon and test harnesses
- core package should still return typed errors and let leaves decorate them

### Shell launch should be typed, not stringly

Идея из deeper shell-bootstrap research:

- `ShellLaunchSpec`, `ShellKind` and launch mode should be explicit domain concepts
- `shell -lc` should not be the public process model

### Discovery is not authority

Идея из `which`:

- discovering an executable path is useful
- but it should not be confused with permission, trust or launch policy

### Embed shell integration assets as versioned resources

Идея из `include_dir` and `rust-embed`:

- shell bootstrap snippets and integration files should be packaged as explicit assets
- they should not live as raw string constants scattered through host-specific code

### Keep shell-like parsing at product edges only

Идея из `shell-words` and `shlex`:

- parsing shell syntax is useful for explicit UX surfaces
- but the runtime launch contract should remain structured

### Foreground ownership should be explicit runtime truth

Идея из job-control research:

- prompt UX, autosuggestions and command-entry assumptions should gate on real foreground ownership
- shell-looking output is not enough

### Host APIs should send signal intents, not Unix signal plumbing

Идея из `nix`, `rustix` and `signal-hook`:

- hosts should speak in `Interrupt`, `Suspend`, `Continue`, `ResizeNotice`
- Unix leaves can translate that into process-group and signal behavior

### PTY/session setup is not enough to model job control

Идея из `portable-pty`, `pty-process` and Unix job-control semantics:

- controlling terminal setup and PTY child launch are necessary
- but foreground process-group ownership is a separate layer

### OSC side effects should become typed intents

Идея из side-effect and OSC research:

- clipboard, hyperlink, notification and status requests should be parsed into typed intents
- executing them should remain a host capability decision

### Notifications and status bridge should stay separate

Идея из `cmux` and host-bridge research:

- transient attention events and long-lived current state are different product concepts
- the runtime should not collapse them into one generic notification channel

### Parse URLs early, open them late

Идея из `url` plus host-side open crates:

- hyperlinks should become typed URL values
- actual open action should stay in capability-gated host adapters

### Inline graphics should stay capability-driven and optional

Идея из media-protocol research:

- `SIXEL`, kitty-style graphics and similar features should be negotiated as optional capabilities
- unsupported hosts should have explicit fallback behavior

### Text terminal truth should stay separate from media rendering

Идея из `icy_sixel`, `ratatui-image` and host-render boundary research:

- image decode/encode and protocol-specific media rendering belong above terminal truth
- widget/viewer crates should not define the minimal runtime contract

### If we pick one protocol-family first, `SIXEL` currently has the strongest Rust bricks

Идея из current ecosystem scan:

- `icy_sixel` and `sixel-rs` make `SIXEL` look like the least speculative Rust-first path today

### Child-visible terminal identity should be explicit policy

Идея из `TERM` / terminfo / capability-advertisement research:

- the package should have an intentional `TerminalIdentityPolicy`
- `TERM`, `COLORTERM` and related markers should come from that policy, not from host accidents

### Host capabilities and child-visible identity must stay separate

Идея из terminal identity research:

- what the host can render and what the child should believe are different contracts
- over-advertising support is compatibility debt

### Terminfo should stay an adapter/helper layer

Идея из `terminfo`, `termini`, `terminfo-lean`, `termprofile` and `termwiz`:

- terminfo data is useful for compatibility and detection
- but it should not be the only truth source for package capability policy

### Stored credentials, secret material and agent access must stay separate

Идея из credential/agent research:

- credential references, in-memory secrets and SSH agent capabilities are different concepts
- treating them as one generic auth blob creates authority confusion

### Agent forwarding should be explicit and deny-by-default

Идея из remote credential boundary research:

- forwarding is delegated authority, not just convenience
- route policy should make it visible and opt-in

### Transport adapters should consume credentials, not define credential truth

Идея из `russh` and `openssh` plus keyring research:

- transport layers should authenticate using typed capability inputs
- storage/keychain policy belongs elsewhere

### Time budgets should be explicit domain policy

Идея из `tokio::time`, `CancellationToken` and `TaskTracker`:

- start, attach, replay catchup, search, export and remote operations should have explicit deadline semantics
- cancellation should be visible and testable, not hidden in helper functions

### One architecture may need multiple product artifact families

Идея из `cargo-dist`, `cargo-c`, `napi-rs` and multi-host distribution research:

- crates, daemon binaries, C ABI packages and Node leaves should be treated as separate product surfaces
- release automation should ship them coherently, but should not define their semantics

### Do not let Electron become the architecture center

Идея из `napi-rs` plus artifact-topology research:

- Electron can be the first host
- but the package truth should still live in Rust crates, protocol contracts and runtime services
- Node addons should stay leaf adapters

### If we ship a C ABI, ship it like a real systems library

Идея из `safer-ffi`, `cbindgen`, `cargo-c` and `rustls-ffi`:

- headers, install story and `pkg-config` metadata are part of the product
- ad hoc `.dylib` or `.so` drops are not a serious reusable ABI surface

### Cross-build tooling solves different jobs

Идея из `cargo-zigbuild` and `cargo-xwin`:

- Unix portability tuning and Windows MSVC cross-builds should be treated as separate lanes
- one "cross tool" should not silently dictate the whole artifact matrix

### Installer convenience is not compatibility truth

Идея из `cargo-binstall`:

- convenient binary install is useful for consumers
- but it should not be confused with release policy, ABI support matrix or host-SDK guarantees

### Trust artifacts should be explicit release outputs

Идея из `cargo-auditable`, `cargo-cyclonedx`, `cargo-vet` and `cargo-about`:

- SBOM, license inventory, dependency trust policy and binary auditability are different artifacts
- they should be generated intentionally from the workspace, not improvised in release notes

### Signing belongs at release leaves, not in core crates

Идея из `sigstore`, `cosign` and `apple-codesign` research:

- provenance and signing should happen on published artifacts
- runtime crates should not absorb signing-key, notarization or installer concerns

### Platform bundles and installers are delivery lanes, not product truth

Идея из `cargo-bundle`, `cargo-deb`, `cargo-generate-rpm`, `cargo-wix` and `cargo-appimage`:

- bundle/install formats should sit above the artifact matrix
- they should not dictate runtime boundaries or host-neutral SDK semantics

### Changelog quality supports trust but does not replace it

Идея из `cargo-release` and `git-cliff`:

- disciplined release notes and version flows matter
- but they are not substitutes for SBOM, provenance or artifact signing

### Internal crate modularity and public publishability are different decisions

Идея из `release-plz`, `guppy` and workspace-publish research:

- many internal crates can be healthy
- but public semver surface should stay much smaller and intentional

### Use facade crates as curated front doors

Идея из multi-crate reusable-platform research:

- facade crates are useful when internal graph is rich
- they should hide churn and curate entry points, not become giant dumping grounds

### Publish host leaves separately

Идея из artifact-topology plus publish-graph research:

- `terminal-node`, `terminal-capi`, `terminal-daemon` and similar leaves should remain distinct publish surfaces
- host-specific concerns should not leak into one giant public crate

### Release coordination should follow the graph, not define it

Идея из `release-plz`, `cargo-workspaces` and `cargo-release`:

- tooling can coordinate multi-crate release flows
- but the public graph must still be an architecture decision first

### Large workspaces need dependency-graph hygiene tools

Идея из `guppy`, `cargo-hakari`, `cargo-machete` and `cargo-deny`:

- build performance, dead deps and graph-policy drift become their own maintenance problem
- this needs explicit tooling, not just reviewer discipline

### Rust-first DTO truth, generated host carriers second

Идея из `ts-rs`, `typeshare` and schema-generation research:

- protocol DTOs should remain intentional Rust-owned contracts
- TS and other language carrier types should be generated from them as leaf artifacts

### Config schemas and live runtime protocols are different categories

Идея из `schemars` plus runtime-protocol research:

- config, manifests and persisted documents can lean on JSON Schema
- streaming runtime envelopes and subscriptions should not be forced into the same model

### `typeshare` is the strongest multi-language watchlist for host carriers

Идея из current ecosystem scan:

- if later we want Swift/Kotlin/TS carrier sync from Rust-owned models, `typeshare` looks like the strongest adjacent option today

### `typify` should stay an ingestion seam

Идея из `typify` research:

- importing external schema into Rust is useful
- but it should not become the main authoring path for our own runtime contracts

### Reflection is useful for compatibility, not for public contract authorship

Идея из `serde-reflection`:

- reflection-derived format knowledge is good for tests and evolution tooling
- but public SDK contracts still need explicit design

### Builders should wrap deliberate spec objects, not replace domain design

Идея из `typed-builder`, `bon` and public-API research:

- builders are useful for rich option/spec structs
- but they should not become the hidden source of truth for the public API

### Seal traits unless downstream implementation is a real product goal

Идея из Rust API Guidelines future-proofing:

- if a public trait is not meant to be implemented by embedders, seal it
- this preserves evolution room for the crate

### Private fields by default on public structs

Идея из Rust API Guidelines:

- representation and invariants should stay hidden unless the type is truly passive data
- this matters a lot for runtime options, routes and capability-bearing types

### Opaque/newtype wrappers are semver tools, not only type-safety tools

Идея из API-guidelines plus earlier handle-model research:

- wrapping IDs and handles preserves semantics and future freedom
- avoid exposing primitive representation where the semantic contract matters

### Public API tooling is a guardrail, not an API design method

Идея из `cargo-public-api` and `cargo-semver-checks`:

- use them to catch drift and breaks
- do not rely on them instead of shaping a future-proof surface first

### Many small runtime machines beat one giant machine

Идея из typestate/FSM research:

- `SessionPhase`, `LaunchState`, `RestoreState`, `ForegroundOwnershipState` and similar concerns should stay separate
- orthogonal lifecycle slices should not be forced into one monolithic FSM

### Typestate is stronger for setup APIs than for live runtime truth

Идея из `typestate` and `state_machine_future` research:

- compile-time transitions are useful for short-lived strict protocols
- long-lived async reconnectable runtime state is usually a weaker fit

### Verification can matter more than runtime DSL elegance

Идея из `stateright`:

- for critical invariants around attach/reconnect/replay/ownership, model checking can be more valuable than prettier macro-generated runtime code

### Use FSM crates only for narrow bounded subsystems

Идея из `state-machines` and `rust-fsm`:

- if a state graph is small, stable and local, an FSM crate may help
- but it should not dictate the whole runtime architecture

### Subscription objects are better public contracts than raw channel types

Идея из Rust async-stream surface research:

- `ScreenSubscription`, `EventSubscription` and similar owned objects should carry lifecycle semantics
- raw `mpsc`/`broadcast` receiver types should remain internal

### `futures-core::Stream` is the best abstraction edge if a stream is needed

Идея из `futures-core` plus Tokio-adapter research:

- keep the public stream boundary trait-light
- avoid freezing Tokio-specific wrappers into the API unless a leaf explicitly wants that

### Raw bytes and semantic events should stay different subscription families

Идея из `bytes` and public async-carrier research:

- high-throughput binary carriers and typed semantic event streams should not collapse into one generic event surface

### Stream trait is not a full lifecycle model

Идея из subscription-lifecycle research:

- explicit close/cancel/invalidated semantics still need owned subscription objects or equivalent handles
- a stream alone is not enough to express that lifecycle clearly

### Async truth first, blocking convenience second

Идея из sync-facade research:

- the runtime should stay canonically async
- if blocking embed ergonomics matter, provide a separate facade leaf rather than distorting the core API

### Separate blocking facade crate is healthier than dual-mode macro magic

Идея из `pollster` and `maybe-async` comparison:

- an explicit blocking leaf keeps docs, semver and lifecycle clearer
- trying to unify sync and async public APIs through macro abstraction is likely to create drift and hidden complexity

### `pollster` is a tactical helper, not an architecture

Идея из `pollster` research:

- good for narrow blocking bridges, tests and examples
- not a whole strategy for long-lived runtime embedding

### Do not promise fake executor neutrality

Идея из `async-compat`, `futures-executor` and Tokio-first runtime reality:

- bridge helpers are useful
- but they should not be used to overstate how runtime-independent the package really is

## P3 - Things To Avoid

### Не строить "умный terminal" через тотальный input interception

Это риск:

- сломать shell behavior
- сломать TUI apps
- сломать keybindings

### Не выбирать foundation только по renderer novelty

Новый renderer сам по себе не решает:

- persistence
- session model
- search/control center
- worktree workflow

### Не делать terminal как isolated modal-only surface

Это оставит нас с "обычным терминалом в окошке", а не с реально полезной feature.

## Кандидатный rollout

### Phase 1

- new feature slice
- foundation selection
- tabs/splits
- search
- links
- shell integration markers
- backpressure

### Phase 2

- persistence
- session auto-discovery
- attention states
- workspace metadata

### Phase 3

- browser/editor adjacency
- command blocks
- automation/session snapshots
- experiments with spatial layout

## Current best composition

Если собирать strongest composition по текущему ресёрчу:

1. foundation - `wterm` или `restty`
2. persistence model - идеи из `zmx`
3. attention UX - идеи из `cmux`
4. workspace/worktree flow - идеи из `Factory Floor`
5. context surfaces - идеи из `OpenCove` / `Nezha`
6. automation/testing - идеи из `termscope`

## Sources

- [wterm](https://github.com/vercel-labs/wterm)
- [restty](https://github.com/wiedymi/restty)
- [zmx](https://github.com/neurosnap/zmx)
- [cmux](https://github.com/manaflow-ai/cmux)
- [Factory Floor](https://github.com/alltuner/factoryfloor)
- [OpenCove](https://github.com/DeadWaveWave/opencove)
- [Nezha](https://github.com/hanshuaikang/nezha)
- [termscope](https://github.com/mwunsch/termscope)
- [JetBrains Terminal: A New Architecture](https://blog.jetbrains.com/idea/2025/04/jetbrains-terminal-a-new-architecture/)
- [VS Code Terminal Shell Integration](https://code.visualstudio.com/docs/terminal/shell-integration)
- [WezTerm Shell Integration](https://wezterm.org/shell-integration.html)
- [xterm.js Flow Control](https://xtermjs.org/docs/guides/flowcontrol/)
