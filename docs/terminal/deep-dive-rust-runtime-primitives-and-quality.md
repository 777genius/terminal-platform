# Deep Dive - Rust Runtime Primitives, Persistence, And Verification

**Проверено**: 2026-04-19

## Зачем этот deep dive

После выбора PTY, emulator и public embed boundary остаётся ещё один важный слой:

**какие supporting crates будут держать runtime платформу в реальности**

Потому что world-class terminal package ломается не только на VT semantics.
Он ломается на:

- framed protocol
- shutdown/cancellation
- queue semantics
- durable state
- telemetry
- verification quality

## Primary Sources

### Runtime and protocol primitives

- [`tokio` README](https://github.com/tokio-rs/tokio/blob/master/README.md)
- [`tokio-util/src/codec/mod.rs`](https://github.com/tokio-rs/tokio/blob/master/tokio-util/src/codec/mod.rs)
- [`tokio-util/src/sync/cancellation_token.rs`](https://github.com/tokio-rs/tokio/blob/master/tokio-util/src/sync/cancellation_token.rs)
- [`tokio-util/src/task/task_tracker.rs`](https://github.com/tokio-rs/tokio/blob/master/tokio-util/src/task/task_tracker.rs)
- [`smol` README](https://github.com/smol-rs/smol/blob/master/README.md)
- [`bytes` README](https://github.com/tokio-rs/bytes/blob/master/README.md)
- [`crossbeam-channel` README](https://github.com/crossbeam-rs/crossbeam/blob/master/crossbeam-channel/README.md)
- [`flume` README](https://github.com/zesterer/flume/blob/master/README.md)

### Persistence and data formats

- [`redb` README](https://github.com/cberner/redb/blob/master/README.md)
- [`rusqlite` README](https://github.com/rusqlite/rusqlite/blob/master/README.md)
- [`heed` README](https://github.com/meilisearch/heed/blob/main/README.md)
- [`sled` README](https://github.com/spacejam/sled/blob/main/README.md)
- [`postcard` README](https://github.com/jamesmunns/postcard/blob/main/README.md)
- [`msgpack-rust` README](https://github.com/3Hren/msgpack-rust/blob/master/README.md)

### Telemetry and verification

- [`tracing` README](https://github.com/tokio-rs/tracing/blob/master/README.md)
- [`loom` README](https://github.com/tokio-rs/loom/blob/master/README.md)
- [`proptest` README](https://github.com/proptest-rs/proptest/blob/main/proptest/README.md)
- [`insta` README](https://github.com/mitsuhiko/insta/blob/master/README.md)
- [`nextest` README](https://github.com/nextest-rs/nextest/blob/main/README.md)

## Freshness signals

### Async/runtime ecosystem

- `tokio 1.52.1` - updated `2026-04-16`, `618M+` downloads
- `tokio-util 0.7.18` - updated `2026-01-04`, `520M+` downloads
- `bytes 1.11.1` - updated `2026-02-03`, `667M+` downloads
- `smol 2.0.2` - updated `2024-09-07`, `15.9M+` downloads
- `async-channel 2.5.0` - updated `2025-07-06`, `238M+` downloads
- `futures-lite 2.6.1` - updated `2025-08-04`, `221M+` downloads
- `async-executor 1.14.0` - updated `2026-02-15`, `116M+` downloads
- `crossbeam-channel 0.5.15` - updated `2025-04-08`, `406M+` downloads
- `flume 0.12.0` - updated `2025-12-08`, `150M+` downloads

### Persistence/serialization

- `redb 4.0.0` - updated `2026-04-02`, `4.4M+` downloads
- `rusqlite 0.39.0` - updated `2026-03-15`, `56.5M+` downloads
- `heed 0.22.1` - updated `2026-04-07`, `3.2M+` downloads
- `sled 1.0.0-alpha.124` - updated `2024-10-11`, `11M+` downloads
- `rmp-serde 1.3.1` - updated `2025-12-23`, `90M+` downloads
- `postcard 1.1.3` - updated `2025-07-24`, `28.5M+` downloads
- `serde_json 1.0.149` - updated `2026-01-06`, `840M+` downloads

### Verification

- `tracing 0.1.44` - updated `2025-12-18`, `550M+` downloads
- `tracing-subscriber 0.3.23` - updated `2026-03-13`, `380M+` downloads
- `loom 0.7.2` - updated `2024-04-23`, `45.8M+` downloads
- `proptest 1.11.0` - updated `2026-03-24`, `113M+` downloads
- `insta 1.47.2` - updated `2026-03-30`, `59.5M+` downloads
- `cargo-nextest 0.9.133` - updated `2026-04-14`, `9.7M+` downloads

## Короткий вывод

🔥 Для terminal runtime мирового уровня supporting stack сейчас выглядит так:

- `Tokio + tokio-util + bytes` как основной runtime/control-plane слой
- `interprocess + custom framed protocol` как local daemon transport
- `CancellationToken + TaskTracker` как shutdown/lifecycle primitives
- `tracing` как mandatory telemetry backbone
- `rusqlite` или `redb` для durable state depending on truth shape
- `loom + proptest + insta + nextest` как verification stack

## Top 3 Runtime Support Stacks

### 1. `Tokio + tokio-util + bytes + tracing`

`🎯 10   🛡️ 9   🧠 6`  
Примерно `5000-11000` строк на правильную integration base.

Почему:

- Tokio сам себя описывает как runtime for reliable, asynchronous and scalable applications
- `tokio-util::codec` уже даёт framing model for `AsyncRead`/`AsyncWrite`
- `CancellationToken` и `TaskTracker` дают очень здоровый shutdown shape
- `bytes` остаётся фактическим стандартом для efficient byte buffers
- `tracing` не зависит от Tokio runtime and fits libraries well

Это мой текущий лучший default.

### 2. `smol` ecosystem + focused async crates`

`🎯 6   🛡️ 7   🧠 7`  
Примерно `5000-10000` строк.

Почему интересно:

- `smol` remains a small and fast runtime
- ecosystem is composable by design
- WezTerm itself uses a lot of `smol`-style building blocks

Где риск:

- для public multi-host SDK ecosystem gravity уже сильно на стороне Tokio
- transport, testing, adapters and operational guidance around Tokio richer

### 3. `Hybrid runtime with Tokio orchestration + sync-focused side channels`

`🎯 8   🛡️ 8   🧠 7`  
Примерно `6000-12000` строк.

Идея:

- async orchestration on Tokio
- sync thread boundaries via `crossbeam-channel`
- durable/background boundaries via daemon model

Это выглядит сильнее, чем пытаться затащить всё в один async abstraction.

## 1. Runtime choice - Tokio vs Smol

## `Tokio`

Tokio README прямо позиционирует себя как:

- fast
- reliable
- scalable

и даёт:

- multithreaded work-stealing scheduler
- reactor over OS event queues
- async networking

Что особенно важно для нас:

- very rich surrounding ecosystem
- stable operational story
- strong primitives for graceful shutdown and framing

### Why I would choose Tokio here

Для reusable terminal runtime с:

- daemon mode
- local socket transport
- Node/Electron host
- future remote/runtime service options

Tokio сейчас даёт лучший ecosystem fit.

## `smol`

`smol` честно и правильно описывает себя как:

- small and fast async runtime
- re-export of smaller async crates

Это очень хороший composition-first ecosystem.  
Но для нашего случая есть проблема:

⚠️ public SDK and multi-host tooling gravity is weaker than around Tokio.

Итог:

✅ `smol` is a strong internal engineering option  
⚠️ `Tokio` is a stronger platform default

## 2. Protocol framing - `bytes + tokio-util::codec`

`tokio-util::codec` source-level docs очень важны.
Они прямо формулируют правильную модель:

- raw IO works with byte sequences
- higher-level code wants meaningful chunks called frames
- `FramedRead` / `FramedWrite` adapt `AsyncRead` / `AsyncWrite` into framed streams

🔥 Это почти идеальный mental model для terminal daemon protocol.

Практический вывод:

- local control plane should not be ad-hoc newline JSON
- use `Bytes` / `BytesMut`
- implement explicit `Decoder` / `Encoder`
- keep protocol frame boundaries first-class

## 3. Shutdown and lifecycle - `CancellationToken + TaskTracker`

This is one of the strongest source-level findings.

### `CancellationToken`

Source shows:

- cloneable token
- child tokens
- `cancelled()` future
- cancellation propagation

Это exactly what a session/workstream runtime needs.

### `TaskTracker`

Source explicitly says it is intended to be used together with `CancellationToken` for graceful shutdown.

Important properties:

- wait until tasks exit
- tasks free memory immediately on exit
- tracker can be cloned
- unlike `JoinSet`, completed task outputs do not accumulate silently

🔥 For terminal runtime this is huge:

- session teardown
- daemon shutdown
- terminal attach worker cleanup
- remote stream worker cleanup

all become much easier to reason about.

## 4. Internal channels - `crossbeam-channel` vs `flume`

## `crossbeam-channel`

README says:

- MPMC channels
- alternative to `std::sync::mpsc`
- more features and better performance
- select support
- locks used sparingly

Это остаётся very strong option for sync thread boundaries.

## `flume`

README says:

- blazingly fast MPMC
- async support
- `Sender` and `Receiver` are `Send + Sync + Clone`
- featureful and ergonomic

Но there is one important warning:

⚠️ the project marks itself as **casual maintenance intended**.

Практический вывод:

- `flume` is attractive ergonomically
- but for a long-lived terminal platform I trust `crossbeam-channel` and Tokio ecosystem more as foundation

## Recommended queue strategy

Я бы делал так:

- async orchestration - Tokio primitives
- sync thread boundaries - `crossbeam-channel` where needed
- avoid centering architecture on `flume`

## 5. Persistence - `redb` vs `rusqlite` vs `heed` vs `sled`

## `redb`

README claims:

- simple, portable, high-performance, ACID embedded key-value store
- pure Rust
- stable file format
- MVCC
- crash-safe by default

Это очень сильный candidate when truth is mostly:

- key/value session state
- durable mirrors
- append-ish terminal artifacts

## `rusqlite`

README positions it as ergonomic wrapper for SQLite from Rust.
Important practical strengths:

- very mature ecosystem
- rich feature surface
- hooks, tracing, virtual tables, backup, serialization
- `bundled` mode solves many build issues

Это strongest choice when truth shape is:

- relational metadata
- indexing
- filtering and querying
- complex restore rules

## `heed`

README positions it as:

- Rust-centric LMDB abstractions with minimal overhead
- safe and ACID

Это interesting high-performance option if you deliberately want LMDB semantics.

## `sled`

This one is the most important cautionary finding.

README explicitly warns:

- README is out of sync with main branch
- large in-progress rewrite
- reliability-first users should use SQLite
- on-disk format will change before `1.0`

🔥 I would not choose `sled` as the primary durable store for this package.

## Storage recommendation

### 1. `rusqlite`

`🎯 9   🛡️ 9   🧠 5`

Best for:

- session metadata
- workspace topology
- restore/index/search metadata

### 2. `redb`

`🎯 8   🛡️ 8   🧠 5`

Best for:

- simple durable runtime truth
- append-ish mirrors
- embedded stable file format without SQLite shape

### 3. `heed`

`🎯 6   🛡️ 8   🧠 7`

Best for:

- LMDB-style high-performance scenarios

### Not recommended as primary

- `sled` - `🎯 3   🛡️ 4   🧠 6`

## 6. Serialization formats - JSON vs MessagePack vs Postcard

## `serde_json`

- enormous adoption
- easiest debugging
- strongest compatibility for host tools

Best for:

- human-readable control plane
- diagnostics
- public command/query payloads

## `rmp-serde`

README highlights:

- compact self-describing binary format
- zero-copy value decoding in the wider project
- extensible without schema files

Best for:

- optional binary transport
- higher-throughput remote protocol

## `postcard`

README highlights:

- no_std-first
- stable wire format
- resource efficiency
- flavor/middleware system

But for our use case:

⚠️ It is optimized for constrained environments, not as the main public protocol for a desktop terminal platform.

## Serialization recommendation

### 1. `JSON control plane + raw PTY byte stream`

`🎯 10   🛡️ 9   🧠 5`

Best default for v1.

### 2. `MessagePack for optional binary protocol`

`🎯 7   🛡️ 8   🧠 6`

Good future option once protocol stabilizes.

### 3. `Postcard`

`🎯 4   🛡️ 8   🧠 6`

Interesting technically, but not my public desktop-host default.

## 7. Telemetry - `tracing`

`tracing` README is very aligned with our needs:

- structured, event-based diagnostics
- applications install collectors/subscribers
- libraries should only depend on `tracing` and must not install global collectors

🔥 This is exactly the rule we need:

- runtime crates emit spans/events
- host apps choose collection/export policy

That maps perfectly to:

- reusable package
- standalone app
- Electron host
- CI/testing tooling

## 8. Verification stack - `loom + proptest + insta + nextest`

## `loom`

Loom tests concurrent Rust code by exploring many concurrent executions under the C11 memory model.

That is extremely valuable for:

- lock-free or low-lock queues
- shutdown races
- replay buffer coordination
- attach/detach ordering

## `proptest`

Proptest gives:

- property testing
- shrinking
- failure persistence

This is ideal for:

- protocol parser fuzzing
- replay merge semantics
- state machine invariants

## `insta`

Snapshot testing is very strong for:

- protocol payloads
- restored terminal projections
- serialized durable state
- diagnostics surfaces

## `cargo-nextest`

Nextest gives:

- faster next-generation Rust test running
- a split between runner/metadata/filtering crates

Это strong CI baseline for a big workspace.

## Recommended quality stack

### 1. `loom + proptest + insta + nextest`

`🎯 10   🛡️ 9   🧠 6`

This is the stack I would actually target.

## Final take

If we translate all of this into one practical recommendation for your future package, I would design supporting infrastructure like this:

- async/runtime: `Tokio`
- framed local protocol: `bytes + tokio-util::codec + interprocess`
- shutdown/lifecycle: `CancellationToken + TaskTracker`
- telemetry: `tracing` and `tracing-subscriber`
- persistence:
  - `rusqlite` for metadata/indexing-oriented truth
  - `redb` for simpler durable mirrors or KV truth
- verification: `loom + proptest + insta + nextest`

🔥 The important meta-rule is:

**choose primitives by truth shape and lifecycle semantics, not by "this crate looks cool".**
