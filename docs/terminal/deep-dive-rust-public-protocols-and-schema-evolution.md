# Deep Dive - Rust Public Protocols, Schema Evolution, And Multi-Host Interop

**Проверено**: 2026-04-19

## Зачем этот deep dive

Когда terminal runtime должен жить в `Rust`, а UI и host app могут быть какими угодно, главный вопрос уже не только:

- какой PTY crate взять
- какой emulator core взять
- какой daemon topology выбрать

Главный вопрос становится таким:

🔥 **какой public contract переживёт годы, много host-ов и много языков**

Именно здесь embeddable runtime-проекты обычно ломаются:

- локальный Node adapter случайно становится источником истины
- FFI surface начинает нести бизнес-семантику сам по себе
- in-process Rust types путают с public compatibility boundary
- control plane, byte stream и persistence truth склеиваются в один ad-hoc protocol

## Primary Sources

- [`wit-bindgen` README](https://github.com/bytecodealliance/wit-bindgen/blob/main/README.md)
- [`wasmtime` README](https://github.com/bytecodealliance/wasmtime/blob/main/README.md)
- [`prost` README](https://github.com/tokio-rs/prost/blob/master/README.md)
- [`prost-reflect` README](https://github.com/andrewhickman/prost-reflect/blob/master/README.md)
- [`jsonrpsee` README](https://github.com/paritytech/jsonrpsee/blob/master/README.md)
- [`tarpc` README](https://github.com/google/tarpc/blob/master/README.md)
- [`capnproto-rust` README](https://github.com/capnproto/capnproto-rust/blob/master/README.md)
- [`FlatBuffers` README](https://github.com/google/flatbuffers/blob/master/README.md)
- [`rkyv` README](https://github.com/rkyv/rkyv/blob/master/README.md)
- [`zerocopy` README](https://github.com/google/zerocopy/blob/master/README.md)
- [`safer_ffi` README](https://github.com/getditto/safer_ffi/blob/master/README.md)

## Freshness Signals

На `2026-04-19` свежие версии и активность по `cargo info` и GitHub такие:

- `wit-bindgen 0.57.1`, repo `1382` stars, push `2026-04-17`
- `wasmtime 43.0.1` latest on crates, repo `17898` stars, push `2026-04-17`
- `prost 0.14.3`, repo `4665` stars, push `2026-03-02`
- `prost-reflect 0.16.3`
- `jsonrpsee 0.26.0`, repo `830` stars, push `2026-04-18`
- `tarpc 0.37.0`, repo `3685` stars, push `2026-03-25`
- `capnp 0.25.4`, `capnp-rpc 0.25.0`, repo `2451` stars, push `2026-04-17`
- `flatbuffers 25.12.19`, repo `25815` stars, push `2026-04-18`
- `rkyv 0.8.15`, repo `4156` stars, push `2026-04-16`
- `zerocopy 0.9.0-alpha.0`, repo `2273` stars, push `2026-04-19`
- `safer-ffi 0.2.0-rc1`, repo `1031` stars, push `2026-04-17`
- `serde_json 1.0.149`
- `rmp-serde 1.3.1`
- `postcard 1.1.3`
- `interprocess 2.4.1`

## Короткий вывод

🔥 Для universal terminal package я бы сейчас проектировал boundary так:

1. **typed control plane**
2. **separate byte stream plane**
3. **optional C ABI layer**
4. **thin host adapters above the same model**

Это автоматически приводит к трём важным правилам:

- public control contract должен быть versioned и host-neutral
- raw PTY stream нельзя смешивать с control/events
- internal zero-copy tricks не должны становиться public protocol truth

## Top 3 Public Contract Strategies

### 1. `Framed control protocol + raw byte/data plane + thin adapters`

`🎯 10   🛡️ 9   🧠 7`  
Примерно `7000-14000` строк до хорошего reusable boundary.

Что это значит:

- локальный daemon или embedded runtime говорит с host-ом через framed protocol
- control plane несёт:
  - `command`
  - `query`
  - `event`
  - `capability`
  - `error`
  - `revision`
- PTY data plane живёт отдельно:
  - input bytes
  - output chunks
  - snapshot / replay frames
  - terminal-generated responses
- Node/Electron, C ABI и future hosts отображают это на свои adapters

Почему это strongest path:

- не завязывает продукт на один язык
- даёт устойчивый путь для daemon mode и in-process embedding
- хорошо переживает смену UI stack
- легко держать Clean Architecture: contract отдельно, adapter отдельно

Практический стек:

- `interprocess`
- `tokio-util::codec`
- `bytes`
- `serde_json` для control plane на ранних этапах
- raw binary frames для stream plane

### 2. `Protobuf/prost schema-first protocol + optional reflection`

`🎯 8   🛡️ 9   🧠 7`  
Примерно `6000-12000` строк.

Что это значит:

- typed public protocol описывается `.proto`
- Rust side использует `prost`
- если нужна dynamic discovery, tooling или bridge to JSON, помогает `prost-reflect`

Почему это сильно:

- schema evolution значительно лучше дисциплинируется
- protobuf хорошо живёт в multi-language мире
- `prost` использует `bytes::{Buf, BufMut}`, а не завязан на `Read/Write`
- `prost` preserves unknown enum values during deserialization
- `prost-reflect` даёт `DescriptorPool`, `DynamicMessage` и JSON mapping

Где риск:

- нужен `protoc`
- control contracts становятся менее удобными для ручной отладки, чем JSON
- для purely local desktop embedding это может оказаться лишней строгостью на v1

Практический вывод:

✅ Сильный вариант для **public remote protocol** или когда заранее важны schema evolution и tooling.  
⚠️ Для local-only first milestone я бы не делал protobuf обязательным everywhere.

### 3. `WIT/component-model boundary`

`🎯 5   🛡️ 7   🧠 9`  
Примерно `8000-16000` строк.

Что это значит:

- public API описывается в `WIT`
- Rust runtime or plugin surfaces работают через component model
- host execution идёт через runtime уровня `wasmtime`

Почему это интересно:

- очень сильная long-term multi-language idea
- WIT даёт language-neutral interface description
- `wasmtime` имеет серьёзную multi-language ecosystem и official C API

Почему это не лучший primary boundary сейчас:

- сам `wit-bindgen` прямо фокусируется на **guest** programs compiled to WebAssembly
- README отдельно говорит, что executing a component in a host **is not managed in this repository**
- для PTY-heavy, long-lived terminal runtime это добавляет ещё один большой runtime layer
- terminal platform превращается не просто в Rust package, а в component-host platform

Практический вывод:

✅ Очень интересно для **plugin/sandbox boundary**.  
⚠️ Не выглядит лучшим primary host-app contract для terminal runtime v1.

## Library-by-library findings

## 1. `prost` - лучший schema-first кандидат для публичного протокола

- `prost 0.14.3`
- generates simple, idiomatic Rust from `proto2` and `proto3`
- использует `bytes::{Buf, BufMut}`
- preserves unknown enum values during deserialization
- intentionally does **not** include runtime reflection itself

Итог:

✅ Брать, если protocol уже должен быть строгим, versioned и реально cross-language.

## 2. `prost-reflect` - сильный усилитель для tooling и dynamic discovery

- `prost-reflect 0.16.3`
- `DescriptorPool`
- `DynamicMessage`
- canonical JSON mapping

Где особенно полезен:

- внешние tools
- dynamic inspectors
- gateway/debug shell
- compatibility tooling around evolving protocol

Итог:

✅ Не нужен обязательно в hot path.  
✅ Очень полезен рядом с protobuf contract.

## 3. `jsonrpsee` - сильный remote/external facade, но не product truth

- `jsonrpsee 0.26.0`
- async client/server
- HTTP/HTTP2/WebSocket
- WASM client support
- custom transports
- middleware

Почему интересен:

- отличный слой для remote automation, diagnostics, external tools
- JSON-RPC envelope делает contract понятнее outside Rust

Почему не делать главным contract:

- terminal runtime всё равно требует richer stream plane than plain request/response
- raw PTY bytes, replay и snapshots неудобно делать "как просто JSON-RPC methods"

Итог:

✅ Отличный façade для external control surfaces.  
⚠️ Не лучший единственный runtime protocol.

## 4. `tarpc` - хороший Rust-internal RPC, слабый public multi-language boundary

- `tarpc 0.37.0`
- code-first service definitions
- pluggable transport
- cascading cancellation
- deadline propagation
- tracing instrumentation

Почему нравится:

- внутрирустовый DX хороший
- cancellation и deadlines очень зрелые

Почему не подходит как primary public contract:

- schema is defined in Rust code, not in neutral IDL
- outside Rust story намного слабее
- business compatibility становится завязана на Rust service definitions

Итог:

✅ Хорошо как internal Rust service layer.  
❌ Не лучший basis для universal package API.

## 5. `capnp` / `capnp-rpc` - technically strong, but heavier than needed for v1

- `capnp 0.25.4`
- `capnp-rpc 0.25.0`
- schema-first
- zero-copy traversal
- protocol evolvability
- RPC support
- no_std / no-alloc support

Почему это впечатляет:

- real distributed-systems mindset
- schema evolution продумана
- interface definitions and RPC already there

Почему это не default choice:

- borrowed reader/builder model сложнее для host apps
- zero-copy value proposition меньше чувствуется на control plane, чем на bulk data
- ecosystem adoption в typical desktop embedding ниже, чем у protobuf/json

Итог:

✅ Сильный niche option, если очень важен strict schema + RPC + zero-copy traversal.  
⚠️ Для terminal control plane v1 я бы не делал его default.

## 6. `FlatBuffers` - good cross-language data format, weak fit for terminal control semantics

- `flatbuffers 25.12.19`
- cross-platform serialization library
- direct access without parsing/unpacking first
- forwards/backwards compatibility
- codegen for many languages

Почему это интересно:

- язык-нейтрально
- эффективно по памяти
- можно генерировать code сразу для Rust/TS/C++/Python

Почему fit ограничен:

- terminal control plane обычно больше страдает от lifecycle semantics, revisioning и stream ownership, чем от parsing cost
- schema/codegen discipline хороша, но ergonomics вокруг interactive protocol хуже, чем у protobuf + explicit envelopes

Итог:

⚠️ Интересно как data format.  
⚠️ Не выглядит самым natural fit для terminal session protocol.

## 7. `rkyv` и `zerocopy` - internal performance tools, not public protocol truth

### `rkyv`

- `rkyv 0.8.15`
- zero-copy deserialization framework for Rust

### `zerocopy`

- `zerocopy 0.9.0-alpha.0`
- safe conversion traits and macros for byte-oriented memory work
- security-focused engineering and strong soundness posture

Почему это полезно:

- hot-path buffers
- snapshots
- internal caches
- binary parsing helpers

Почему это не public contract:

- `rkyv` - Rust-centric archive model
- `zerocopy` - low-level Rust memory/layout helper, not a cross-language schema system
- neither should define the public compatibility story of the package

Итог:

✅ Excellent internal building blocks.  
❌ Не выносить ими public API наружу.

## 8. `wit-bindgen` + `wasmtime` - лучше для plugin/sandbox story, чем для host boundary

### `wit-bindgen`

- WIT and component model bindings generator
- repo explicitly focused on **guest** programs compiled to WebAssembly

### `wasmtime`

- serious standalone WebAssembly runtime
- component-model feature
- official C API
- language embeddings for Rust, C, Python, .NET, Go, Ruby

Почему связка сильная:

- language-neutral interface model
- serious execution/runtime ecosystem

Почему я бы не ставил её в центр terminal package:

- terminal runtime сам по себе уже сложный system service
- component model adds another runtime platform and deployment story
- plugin boundary и host-app boundary лучше не смешивать

Итог:

✅ Keep in mind for plugin sandboxing or extension model.  
⚠️ Не ставить в центр v1 host embedding architecture.

## 9. `safer-ffi` - сильный deliberate C ABI path

- `safer-ffi 0.2.0-rc1`
- helps write FFI without polluting Rust code with `unsafe`
- supports header generation flow

Почему важно:

- C ABI остаётся лучшим universal fallback
- deliberate low-level boundary всегда полезен для serious embeddability

Ограничение:

- current latest is still `rc`
- async/runtime/session semantics всё равно надо проектировать поверх ABI, а не ожидать, что FFI framework решит их за нас

Итог:

✅ Отличный secondary boundary.  
⚠️ Не путать с главным product contract.

## Самые важные architectural выводы

### 1. Separate control plane from byte/data plane

Control plane:

- lifecycle commands
- topology
- attach/replay
- capability negotiation
- status/events

Data plane:

- PTY input bytes
- PTY output chunks
- terminal-generated responses
- snapshots and replay frames

🔥 Это важнее, чем выбор между protobuf и JSON.

### 2. Public compatibility must not depend on Rust types

- Rust structs are internal model
- public compatibility lives in protocol schema and versioning rules
- host bindings should map onto this, not replace it

### 3. Prefer schema-first or envelope-first over code-first for public APIs

`tarpc`-style code-first definition очень удобна внутри Rust, но плохо масштабируется как any-language product contract.

### 4. Zero-copy is an optimization, not the first design driver

Для terminal platform обычно важнее:

- ordering
- lifecycle
- replay semantics
- cancellation
- versioning
- debuggability

чем максимальная theoretical parsing speed control messages.

### 5. Component-model is probably a future extension seam, not the core host seam

Это сильная future-facing investment, но не shortest path to a world-class embeddable terminal runtime.

## Current Practical Recommendation

Если выбирать сейчас, я бы делал так:

1. **Primary host-neutral contract**
   - framed local protocol
   - explicit envelope model
   - JSON control plane on v1
   - raw binary data plane for PTY/snapshot/replay

2. **Optional stricter schema path**
   - evolve toward `prost` for remote/public protocol if ecosystem pressure appears
   - use `prost-reflect` for tooling and compatibility inspection

3. **Secondary boundaries**
   - C ABI via `safer-ffi` / `cbindgen` / `cargo-c`
   - Node/Electron adapter via `napi-rs`

4. **Keep separate for later**
   - `jsonrpsee` as remote/external facade
   - WIT/component-model for plugin or sandbox boundaries
   - `rkyv` / `zerocopy` as internal performance helpers only

## Sources

- [wit-bindgen](https://github.com/bytecodealliance/wit-bindgen)
- [wasmtime](https://github.com/bytecodealliance/wasmtime)
- [prost](https://github.com/tokio-rs/prost)
- [prost-reflect](https://github.com/andrewhickman/prost-reflect)
- [jsonrpsee](https://github.com/paritytech/jsonrpsee)
- [tarpc](https://github.com/google/tarpc)
- [capnproto-rust](https://github.com/capnproto/capnproto-rust)
- [FlatBuffers](https://github.com/google/flatbuffers)
- [rkyv](https://github.com/rkyv/rkyv)
- [zerocopy](https://github.com/google/zerocopy)
- [safer_ffi](https://github.com/getditto/safer_ffi)
