# Deep Dive - Rust Public Async Streams, Subscriptions, and Carriers

**Проверено**: 2026-04-19

## Зачем этот слой смотреть отдельно

Предыдущие deep dive уже зафиксировали важные вещи:

- operations и subscriptions должны быть разными публичными концепциями
- runtime владеет truth и cancellation
- host SDKs не должны владеть runtime graph

Но для reusable Rust package этого мало.  
Нужно отдельно решить:

- что именно должен видеть Rust embedder
- надо ли выдавать `Stream`
- если да, то какой именно trait должен быть public boundary
- можно ли светить `tokio::sync::*` и `tokio_stream::*` типы наружу
- где полезны `bytes`, `pin-project-lite`, `async-stream`
- где кончается internal async plumbing и начинается public API

🔥 Здесь легко сделать плохой выбор:

- засветить `broadcast::Receiver` или `mpsc::Receiver` как public truth
- привязать библиотеку к Tokio-shaped API сильнее, чем нужно
- сделать один generic `Stream<Item = Event>` для всего и потерять lifecycle semantics
- выдать слишком concrete stream wrappers и зацементировать transport/backpressure details

Для вашего terminal package это критично, потому что Rust embedders - это ещё один first-class consumer рядом с JS/Electron и C ABI.

## Primary Sources

### Async and stream primitives

- [`tokio` crate](https://crates.io/crates/tokio)
- [`tokio` repo](https://github.com/tokio-rs/tokio)
- [`tokio-stream` crate](https://crates.io/crates/tokio-stream)
- [`futures-core` crate](https://crates.io/crates/futures-core)
- [`futures-util` crate](https://crates.io/crates/futures-util)
- [`futures-rs` repo](https://github.com/rust-lang/futures-rs)
- [`async-stream` crate](https://crates.io/crates/async-stream)
- [`async-stream` repo](https://github.com/tokio-rs/async-stream)
- [`pin-project-lite` crate](https://crates.io/crates/pin-project-lite)
- [`pin-project` repo](https://github.com/taiki-e/pin-project)
- [`bytes` crate](https://crates.io/crates/bytes)
- [`bytes` repo](https://github.com/tokio-rs/bytes)
- [`tokio-util` crate](https://crates.io/crates/tokio-util)

## Freshness signals

- `tokio 1.52.1` - repo `tokio-rs/tokio`, `31697` stars, pushed `2026-04-18`
- `tokio-stream 0.1.18`
- `tokio-util 0.7.18`
- `futures-core 0.3.32`
- `futures-util 0.3.32` - repo `rust-lang/futures-rs`, `5842` stars, pushed `2026-04-12`
- `async-stream 0.3.6` - repo `tokio-rs/async-stream`, `751` stars, pushed `2024-12-08`
- `pin-project-lite 0.2.17` - repo `taiki-e/pin-project`, `706` stars, pushed `2026-04-15`
- `bytes 1.11.1` - repo `tokio-rs/bytes`, `2205` stars, pushed `2026-02-04`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**public Rust async surface should be handle-first and trait-light, while concrete async carriers stay mostly internal**

Healthiest shape сейчас выглядит так:

1. operations remain explicit API calls
2. subscriptions remain explicit owned resources
3. if Rust consumers need streaming, expose narrow stream facades or adaptors
4. prefer `futures-core::Stream` at the abstraction edge over Tokio-specific concrete types
5. keep channel types, task wiring and backpressure carriers internal

То есть не:

- "public API = just give them a `tokio::sync::mpsc::Receiver`"

а:

- explicit subscription objects
- explicit close/cancel
- narrow stream view when useful
- concrete plumbing hidden

## Top 3 directions for Rust-facing async surface

### 1. `Handle-first API + optional narrow Stream facades`

`🎯 10   🛡️ 9   🧠 7`
Примерно `7000-14000` строк.

Это strongest default.

Идея:

- API returns `SubscriptionHandle` or typed subscription object
- object may offer:
  - explicit `close`
  - maybe `recv_next`
  - maybe a narrow `impl Stream<Item = ...>` or boxed stream adaptor
- runtime still owns channels and tasks internally

Почему это лучший путь:

- preserves lifecycle semantics
- avoids leaking Tokio internals
- still ergonomic for Rust async consumers
- fits earlier ownership/cancellation decisions

### 2. `Tokio-native public stream surface`

`🎯 7   🛡️ 7   🧠 6`
Примерно `5000-11000` строк.

Идея:

- library openly exposes Tokio-shaped async types
- uses `tokio-stream`, receiver wrappers and Tokio semantics more directly

Почему это иногда работает:

- easy for Tokio-first users
- less wrapper code
- ecosystem familiarity

Почему это weaker here:

- stronger executor/runtime coupling
- harder future non-Tokio story
- easier to freeze internal channel choices into public contract

### 3. `Concrete channels as public API`

`🎯 3   🛡️ 4   🧠 4`
Примерно `3000-8000` строк на быстрый старт и потом дорого чинить.

Это плохой path.

Симптомы:

- `mpsc::Receiver` becomes the API
- `broadcast::Receiver` quirks become product semantics
- backpressure/lag/drop policy gets frozen accidentally
- cancellation and close semantics drift into channel implementation details

## 1. `futures-core::Stream` is a better abstraction edge than Tokio-specific stream types

This is the key abstraction lesson.

`futures-core::Stream` gives:

- small dependency surface
- minimal trait contract
- less Tokio lock-in

That makes it a better abstraction edge when we want to expose a stream-like concept without freezing transport details.

Healthy role:

- trait bound or returned adaptor
- public API abstraction seam

Less healthy role:

- the one and only story for subscriptions

Because:

- subscription lifecycle still matters
- stream alone does not express explicit close/cancel/ownership semantics

## 2. `tokio-stream` is useful adapter glue, but should not become public truth

`tokio-stream 0.1.18` is clearly useful.

Why:

- bridges Tokio types into stream world
- good ecosystem ergonomics
- practical wrappers for receiver-like internals

But the strong rule stays:

- `tokio-stream` is best as an adapter/helper layer
- not as the conceptual center of the public API

Good role:

- internal or leaf-facing adaptors
- maybe convenience methods behind feature gates

Bad role:

- making every public subscription literally a `tokio_stream` wrapper

## 3. `futures-util` is ergonomic, but it is not the minimal contract

`futures-util 0.3.32` is very useful for implementation.

Good at:

- combinators
- adaptors
- utility traits

But for public API design:

- it is larger than `futures-core`
- it expresses more ecosystem commitment than is always needed

Healthy pattern:

- `futures-core` at the boundary
- `futures-util` in implementation or optional convenience layers

## 4. `async-stream` is strong internal glue, not a public shape

`async-stream 0.3.6` is elegant for turning internal async logic into stream output.

That is very useful for:

- projections
- subscription adapters
- testing and harnesses
- internal event generation

But the API lesson is:

- do not make public semantics depend on hidden `async-stream` macro shape

Good role:

- internal construction tool

Bad role:

- public API identity

## 5. `pin-project-lite` is exactly the kind of internal primitive that should stay hidden

`pin-project-lite 0.2.17` is great.

Why:

- lightweight
- pragmatic
- good for custom stream/future wrappers

But it reinforces the same boundary lesson:

- projection/pinning mechanics are implementation detail
- embedders should not feel them as the shape of the library

This is a good example of:

- important internal library brick
- zero need to expose it as part of API story

## 6. `bytes` should dominate hot payload carriers, but not necessarily every public event type

`bytes 1.11.1` remains one of the strongest runtime bricks.

Good role:

- hot payloads
- replay chunks
- protocol frames
- binary output paths

But not every Rust-facing event type should be raw `Bytes`.

Healthy split:

- binary/hot data uses `Bytes`
- semantic events use typed DTOs
- stream surface distinguishes the two

🔥 Practical rule:

**do not collapse semantic subscriptions and raw byte subscriptions into one `Stream<Item = Bytes>` story**

## 7. Public subscriptions should be objects, not raw streams

This is the most important synthesis with earlier ownership work.

Why:

- streams do not by themselves express ownership
- streams do not by themselves express explicit close
- streams do not by themselves express stale/invalidated subscription identity

Healthiest shape:

- `ScreenSubscription`
- `EventSubscription`
- `TranscriptSubscription`

Each may offer:

- explicit `close()`
- maybe a `next()` async method
- maybe stream adapter view
- maybe metadata like subscription id or lag state

This keeps:

- lifecycle semantics explicit
- Rust ergonomics good
- channel internals hidden

## 8. `tokio-util` belongs more to internal transport/plumbing than to public subscription shape

`tokio-util 0.7.18` is still essential elsewhere:

- codec
- framing
- runtime helpers

But for this layer it reinforces:

- not every useful async crate belongs in the public API story

Its role here is mostly:

- internal plumbing
- internal transport adaptation

## 9. Recommended shape for this package

### Strong default

- explicit subscription handle/object types
- narrow `futures-core::Stream` adaptors where useful
- `bytes::Bytes` for raw/high-throughput carriers
- typed DTO streams for semantic events
- Tokio channels hidden inside owner-task runtime

### Good internal helpers

- `tokio-stream`
- `futures-util`
- `async-stream`
- `pin-project-lite`

### Avoid exposing directly

- `tokio::sync::mpsc::Receiver`
- `tokio::sync::broadcast::Receiver`
- ad hoc channel types as public subscription contract

## 10. If I were designing this layer right now

- `Session::subscribe_screen()` returns a `ScreenSubscription`
- `ScreenSubscription` has explicit `close`
- `ScreenSubscription` can expose a stream adaptor for Rust async consumers
- raw terminal bytes and semantic runtime events stay different subscription families
- `futures-core::Stream` is the abstraction edge if we need one
- Tokio-specific wrappers remain implementation detail or optional convenience leaves

## Things to avoid

- ❌ Public API equals internal channel type
- ❌ One generic event stream for every domain
- ❌ Forcing all Rust embedders to think in Tokio-specific wrappers
- ❌ Losing explicit close/cancel semantics because "a stream already exists"
- ❌ Treating `Stream` trait as a complete lifecycle model

## Final verdict

🔥 For this terminal package, the healthiest Rust-facing async surface is:

- handle-first
- subscription-object-first
- narrow stream adaptors second
- concrete async carriers hidden
- raw bytes and semantic events kept separate

That gives Rust embedders an ergonomic async story without freezing internal Tokio/channel decisions into the public contract.

## Sources

- [tokio](https://github.com/tokio-rs/tokio)
- [futures-rs](https://github.com/rust-lang/futures-rs)
- [async-stream](https://github.com/tokio-rs/async-stream)
- [pin-project](https://github.com/taiki-e/pin-project)
- [bytes](https://github.com/tokio-rs/bytes)
