# Deep Dive - Rust Host Data Plane, Delta Transport, and Wire Contracts

**Проверено**: 2026-04-19

## Зачем смотреть этот слой отдельно

Для universal terminal package одного "public protocol" недостаточно.

На практике у вас почти сразу появляется несколько разных wire-задач:

- control commands от host к runtime
- session/events/state updates обратно
- screen snapshots and screen deltas
- replay/snapshot transfer
- durable export/debug payloads
- возможно позже multi-language SDKs и remote facades

Если всё это загнать в один формат и один transport shape, обычно ломается либо ergonomics, либо performance, либо evolvability.

Правильный вопрос тут не "JSON или бинарь", а:

- где control plane
- где data plane
- где durable/export format
- какой слой должен быть human-debuggable, а какой hot-path efficient

## Топ 3

### 1. `Framed control plane + explicit binary data plane`
`🎯 10   🛡️ 9   🧠 8`  
Примерно `7000-14000` строк.

Это strongest default.

Идея:

- control plane идёт отдельными framed envelopes
- data plane для screen deltas, snapshots, replay chunks идёт binary-first
- control/event envelopes могут оставаться text-friendly на раннем этапе
- payload types для hot path не прячутся в giant generic serialization layer

Что сюда хорошо ложится:

- `bytes 1.11.1`
- `tokio-util 0.7.18`
- `serde_json 1.0.149` для раннего control plane
- потом optional hardening через `prost 0.14.3`

Почему это лучший путь:

- отлично debug-ится
- host-neutral
- не заставляет screen deltas выглядеть как giant JSON blobs
- не запирает весь пакет на одном schema framework

### 2. `Protobuf/prost for typed public envelopes + separate raw chunk lanes`
`🎯 8   🛡️ 9   🧠 8`  
Примерно `8000-15000` строк.

Это хороший путь, когда protocol реально стабилизируется и появятся:

- external SDKs
- stronger versioning guarantees
- formal schema lifecycle
- multiple hosts and languages with codegen expectations

Почему это сильно:

- `prost 0.14.3` остаётся strong default для protobuf в Rust
- Protobuf нормально живёт в multi-language world
- explicit schemas помогают дисциплине contract evolution

Почему не strongest v1 default:

- screen delta/hot data plane всё равно лучше держать отдельно
- codegen/tooling adds friction
- protobuf не делает за вас хороший terminal transport shape

### 3. `Zero-copy schema families` через `flatbuffers` или `capnp`
`🎯 6   🛡️ 7   🧠 9`  
Примерно `10000-18000` строк.

Это интересный путь, если приоритеты такие:

- very strict cross-language schema
- zero-copy-ish traversal
- long-lived binary protocol commitment

Почему они интересны:

- `flatbuffers 25.12.19` прямо про cross-platform memory-efficient serialization
- `capnp 0.25.4` прямо позиционируется как type system for distributed systems with evolvability and zero-copy traversal

Почему это не лучший default:

- complexity выше
- host ergonomics хуже для быстрого product iteration
- для terminal data plane полезнее explicit split plane design, чем ставка на один sophisticated schema framework

## Самый важный вывод

🔥 У reusable terminal package должно быть минимум **3 wire layers**:

1. `Control plane`
2. `Hot data plane`
3. `Durable/export formats`

И они не обязаны использовать один и тот же encoding.

## Что показал `prost`

- Latest checked: `0.14.3`
- Repo: `tokio-rs/prost`
- Updated: `2026-04-19`

Полезные сигналы:

- protobuf remains strong public schema choice
- generated code simple and idiomatic
- uses `bytes::{Buf, BufMut}` abstractions
- preserves unknown enum values

Очень важный practical insight:

- `prost` хорош именно как typed contract layer
- но он не заменяет explicit split между control envelopes и raw data chunks

Ещё одна деталь:

- README openly marks project as passively-maintained

Это не red flag само по себе. Для protocol tooling такая предсказуемость даже может быть плюсом. Но это значит, что не стоит надеяться, что `prost` magically решит все product-specific transport needs.

## Что показали `flatbuffers` и `capnp`

### `flatbuffers`

- Latest checked: `25.12.19`
- Repo: `google/flatbuffers`
- Updated: `2026-04-19`

README очень явно про:

- maximum memory efficiency
- direct access without unpacking
- forwards/backwards compatibility

Это делает `flatbuffers` интересным для:

- very committed binary public contracts
- cross-language SDKs
- zero-copy-ish host readers

Но для terminal package есть practical issue:

- screen delta model всё равно придётся спроектировать вручную
- schema sophistication не заменяет правильную runtime ownership model

### `capnp`

- Latest checked: `0.25.4`
- Repo: `capnproto-rust`
- Updated: `2026-04-18`

README очень явно позиционирует его как:

- type system for distributed systems
- zero-copy traversal
- interfaces + RPC story
- protocol evolvability

Это сильно, если пакет сразу строится как network-native multi-service platform.

Но для v1 embeddable terminal package я бы был осторожен:

- это уже более opinionated ecosystem choice
- complexity and tooling cost выше
- local-first host boundary не становится от этого автоматически лучше

## Что показали `rmp-serde`, `postcard` и `ciborium`

### `rmp-serde`

- Latest checked: `1.3.1`
- Repo: `3Hren/msgpack-rust`
- Updated: `2026-04-19`

Почему интересно:

- compact self-describing binary format
- good serde ergonomics
- safe concatenation/stream reading fits framed transport stories

Где уместно:

- internal/durable payloads
- debug/export blobs
- maybe internal side channels

Где не лучший default:

- weaker public any-language contract discipline than protobuf schemas
- terminal platform такого уровня не стоит строить вокруг serde-derived public truth

### `postcard`

- Latest checked: `1.1.3`
- Repo: `jamesmunns/postcard`
- Updated: `2026-04-18`

Очень важный сигнал:

- documented stable wire format
- resource-efficient
- strong for constrained/no_std environments

Но это скорее отличный internal or embedded protocol tool, чем лучший public host contract для desktop/multi-language terminal platform.

Почему:

- serde-centric
- ecosystem story для arbitrary host SDKs weaker than protobuf/JSON
- design center там явно не rich desktop host boundary

### `ciborium`

- Latest checked: `0.2.2`
- Repo: `enarx/ciborium`
- Updated: `2026-04-17`

CBOR family интересна как middle ground between JSON and MessagePack, но здесь она не даёт настолько явного выигрыша, чтобы вытеснить:

- `serde_json` для early debuggable control plane
- `prost` для hardened public schemas
- explicit binary chunk lanes for hot data

## Что показали `bytes` и `tokio-util`

### `bytes`

- Latest checked: `1.11.1`
- Repo: `tokio-rs/bytes`
- Updated: `2026-04-17`

`bytes` всё сильнее подтверждается как фундамент именно для hot lanes:

- frame buffers
- replay chunks
- snapshot transport
- explicit boundaries between metadata and payload

### `tokio-util`

- Latest checked: `0.7.18`
- Repo: `tokio-rs/tokio`

`tokio-util::codec` всё ещё выглядит strongest baseline для framed local transport:

- lets you own the envelope format directly
- не прячет protocol under heavyweight RPC assumptions

## Practical verdict

Если выбирать прямо сейчас, я бы делал так:

### V1

- local host boundary: framed protocol over `interprocess`
- control plane: `serde_json` envelopes
- hot data plane: explicit binary chunks over `bytes` + `tokio-util::codec`
- durable/export payloads: choose per surface, not globally

### V2

- once public SDK contract hardens:
  - move stable typed envelopes toward `prost`
  - keep hot screen/replay lanes explicitly binary

## Чего я бы избегал

- ❌ Одинаковый format/framework for control, deltas, replay and exports
- ❌ Giant JSON blobs for screen deltas
- ❌ Flatbuffers/Cap'n Proto just because they look advanced
- ❌ Serde-derived public truth as the only cross-language contract
- ❌ Letting schema framework dictate runtime architecture

## Architecture implications

Для нашего будущего Rust package отсюда следует:

- control plane should be versioned and explicit
- hot data plane should be binary and chunk-oriented
- durable/export formats may differ from live transport
- public host SDK should target stable contracts, not Rust-native internals
- zero-copy frameworks can be future tools, not default architectural center

## Sources

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
