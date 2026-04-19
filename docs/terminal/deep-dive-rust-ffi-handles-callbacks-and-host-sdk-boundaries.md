# Deep Dive - Rust FFI Handles, Callbacks And Host SDK Boundaries

**Проверено**: 2026-04-19

## Зачем этот deep dive

После предыдущих Rust deep dive уже стало ясно, что:

- runtime truth должен жить в Rust
- public contract должен быть protocol-first
- Node/Electron не должен становиться источником product semantics

Но этого ещё мало для world-class embeddable terminal package.

Остаётся самый скользкий слой:

🔥 **как именно host apps на других языках должны держать handles, получать события и жить с lifecycle/runtime semantics**

Именно здесь многие сильные Rust библиотеки начинают деградировать в один из плохих сценариев:

- наружу утекают internal pointers или storage keys
- callback API становится главным contract
- Node adapter случайно становится канонической моделью
- async/event ordering начинает зависеть от FFI glue, а не от runtime

Для terminal package это особенно опасно, потому что у нас long-lived sessions, attach/detach, hot streams, snapshots, deltas, terminal replies и потенциально несколько одновременных host surfaces.

## Primary Sources

- [`getditto/safer_ffi` README](https://github.com/getditto/safer_ffi/blob/master/README.md)
- [`rust-diplomat/diplomat` README](https://github.com/rust-diplomat/diplomat/blob/main/README.md)
- [`mozilla/uniffi-rs` README](https://github.com/mozilla/uniffi-rs/blob/main/README.md)
- [`ralfbiedert/interoptopus` README](https://github.com/ralfbiedert/interoptopus/blob/master/README.md)
- [`napi-rs/napi-rs` `crates/napi/README.md`](https://github.com/napi-rs/napi-rs/blob/main/crates/napi/README.md)
- [`mozilla/cbindgen` README](https://github.com/mozilla/cbindgen/blob/main/README.md)

## Короткий вывод

🔥 Для reusable terminal package правильный public host boundary выглядит так:

1. **runtime truth and protocol semantics live in Rust**
2. **public hosts see opaque handles and explicit envelopes**
3. **events are drained through a deliberate event pump or framed stream**
4. **callbacks are adapter-local convenience, not product truth**
5. **Node/Electron, C ABI and generated SDKs are just different leaves over the same model**

Иначе получится либо FFI soup, либо один очень удобный Node addon, который уже нельзя честно назвать universal terminal platform.

## Top 3 Host SDK Strategies

### 1. `Protocol-first core + opaque handles + explicit event pump + thin adapters`

`🎯 10   🛡️ 9   🧠 8`  
Примерно `8000-16000` строк до сильной reusable boundary.

Что это значит:

- core runtime владеет session/workstream truth
- наружу выдаются:
  - opaque public IDs
  - typed commands/queries
  - typed event envelopes
  - explicit snapshot/delta/data-plane surfaces
- host либо:
  - говорит по local protocol
  - либо вызывает thin in-process adapter над тем же contract
- callbacks остаются только как adapter sugar

Почему это strongest path:

- одинаково хорошо ложится на JS/Electron, C/C++, Python, Swift и future standalone host
- event ordering живёт в runtime, не в binding layer
- multi-client attach/detach/reconnect story не приходится придумывать заново в каждом языке
- легко сохраняется Clean Architecture boundary between domain and adapters

Где риск:

- нужно сразу продумать ownership, event envelopes и lifecycle contract
- чуть дороже старт, чем "просто экспортировать функции в Node"

Практический вывод:

✅ Это мой лучший recommendation для вашей цели.

### 2. `Stable core + generated host SDK layer` через `Diplomat` / `UniFFI`

`🎯 8   🛡️ 8   🧠 7`  
Примерно `6000-12000` строк.

Что это значит:

- core и host-neutral contract уже существуют отдельно
- generator layer выпускает idiomatic SDKs для части языков
- generated wrappers не владеют product semantics, а лишь мапят их наружу

Почему это интересно:

- быстрее даёт multi-language ergonomics
- снижает объём ручного adapter glue
- хорошо работает, если хотите позже SDKs для Python/Kotlin/Swift/TS

Где риск:

- генератор начинает ограничивать форму object model
- long-lived streams, callbacks, async lifecycle и binary hot lanes всё равно нужно проектировать руками

Практический вывод:

✅ Хороший второй слой поверх стабильного core contract.  
⚠️ Плохой первичный фундамент.

### 3. `Binding-direct APIs` с большим количеством callback registration

`🎯 4   🛡️ 6   🧠 5`  
Примерно `3000-8000` строк на быстрый старт и потом дорогое исправление.

Что это значит:

- host language получает direct function surface
- события идут через callbacks
- ordering, lifetime и backpressure начинают зависеть от binding glue

Почему это заманчиво:

- быстро выглядит "нативно" для конкретного host
- demo получается легко

Почему это плохой universal path:

- callback hell быстро становится product contract
- разъезжается модель между Node, C, Python и другими hosts
- тяжелее делать multi-client, reattach, polling, recovery и tooling

Практический вывод:

❌ Для v1 world-class package я бы этот путь не брал.

## Tool-by-tool findings

## 1. `safer-ffi` - strongest deliberate C ABI foundation

- Crate: [`safer-ffi`](https://crates.io/crates/safer-ffi)
- Latest: `0.2.0-rc1`
- Repo stars: `1032`
- Repo updated: `2026-04-19`

Что особенно важно:

- помогает держать C ABI layer явным, а не размазанным по `unsafe extern`
- хорошо ложится на отдельный adapter crate
- сочетается с deliberate header/install story

Что не надо от него ожидать:

- он не спроектирует за нас session lifecycle
- он не решит event ordering
- он не заменит protocol design

Итог:

✅ Лучший кандидат, если даём серьёзный C ABI.

## 2. `Diplomat` - самый интересный future multi-language SDK generator

- Crate: [`diplomat`](https://crates.io/crates/diplomat)
- Latest: `0.15.0`
- Repo stars: `766`
- Repo updated: `2026-04-17`

Что особенно важно:

- реально целится в несколько языков сразу
- прямо перечисляет `C`, `C++`, `Dart`, `Javascript/Typescript`, `Kotlin`, `Python`
- стратегически лучше подходит под "universal package", чем многие генераторы с одной сильной mobile-story

Где осторожность:

- это всё ещё generator layer
- сложные runtime semantics типа event pump, replay, snapshots и hot binary lanes он сам не сформирует

Итог:

✅ Самый интересный generator path на вырост.  
⚠️ Но только поверх already-correct core boundary.

## 3. `UniFFI` - сильный adapter generator, но не лучший центр для terminal runtime

- Crate: [`uniffi`](https://crates.io/crates/uniffi)
- Latest: `0.31.1`
- Repo stars: `4511`
- Repo updated: `2026-04-19`

Что особенно важно:

- production use real
- сильный путь для `Kotlin`, `Swift`, `Python`, `Ruby`
- хорош для object-model-driven SDKs

Где ограничение:

- Node story third-party
- advanced cases сами авторы честно описывают как потенциально ломкие до `1.0`
- terminal runtime с long-lived sessions and streaming сложнее обычного SDK

Итог:

✅ Полезный adapter layer.  
⚠️ Не мой default architectural center.

## 4. `Interoptopus` - сильный donor для services/callbacks/async boundary

- Crate: [`interoptopus`](https://crates.io/crates/interoptopus)
- Latest observed: `0.16.0-alpha.20`
- Repo stars: `460`
- Repo updated: `2026-04-19`

Что особенно важно:

- хорошие идеи around services, callbacks, async and idiomatic errors
- useful as a design donor for how not to flatten everything into plain C calls

Где ограничение:

- maturity ещё неровная
- ecosystem trust ниже, чем у более established tools

Итог:

✅ Хороший R&D donor.  
⚠️ Не мой first-line default для public package.

## 5. `napi-rs` - идеальный Node/Electron leaf adapter

- Crate: [`napi`](https://crates.io/crates/napi)
- Latest: `3.8.5`
- Repo stars: `7683`
- Repo updated: `2026-04-19`

Что особенно важно:

- зрелый Node-specific adapter stack
- хороший async story
- отличная practical fit для Electron embedding

Но:

- это boundary только для Node world
- если канонический session API живёт тут, пакет уже не host-neutral

Итог:

✅ Брать для `terminal-node`.  
❌ Не делать public truth.

## 6. `cbindgen` - обязательный packaging tool, не архитектурный центр

- Crate: [`cbindgen`](https://crates.io/crates/cbindgen)
- Latest: `0.29.2`

Что особенно важно:

- нужен для serious C/C++ integration story
- хорошо дополняет `safer-ffi`

Но:

- header generation не равна API design

Итог:

✅ Essential tool.  
❌ Не стратегия.

## Какой host contract должен видеть мир

## 1. Public handles должны быть opaque

Host не должен видеть:

- raw pointers to emulator internals
- `slotmap` keys
- slab indices
- references на внутренние grid/session objects

Host должен видеть только:

- stable public `SessionId`
- stable public `PaneId`
- stable public `SubscriptionId`
- explicit `RuntimeRouteId` where needed

Практически это обычно значит:

- protocol surfaces используют `UUID`/`ULID`/opaque strings
- C ABI surfaces используют small opaque value types or handles
- host adapters never expose internal storage keys

Это согласуется с уже зафиксированным state model rule из `slotmap` research.

## 2. Callback registration не должен быть главным event model

Самая частая FFI ошибка:

- host вызывает `set_on_output(callback)`
- дальше весь runtime implicitly depends on callback timing and host liveness

Для terminal platform это плохо, потому что:

- output и state changes идут разными классами трафика
- callbacks не дают хороший replay contract
- detach/reattach/multi-client story становится хрупкой

Лучший shape:

- primary event source = explicit event queue / framed stream / pollable drain
- callback API, если есть, висит как thin adapter above event queue

То есть:

- runtime produces envelopes
- adapter drains envelopes
- adapter may re-emit them as callbacks/promises/events in the host language

Но runtime truth не зависит от foreign callback.

## 3. Event pump лучше, чем "магические async callbacks"

Для reusable SDK полезнее всего выглядят две модели:

### A. `poll_events(max_items)` / `next_event(timeout)`

Сильные стороны:

- простой universal contract
- хорошо ложится на C ABI
- легко тестировать
- хороший control over backpressure

Где неудобно:

- host сам должен крутить loop

### B. framed subscription stream

Сильные стороны:

- лучше для local daemon and sockets
- естественно для Node async iterators / streams
- удобно для viewer/controller split

Где неудобно:

- сложнее C ABI embedding

Практический вывод:

✅ Пакет мирового уровня должен уметь оба shape:

- protocol/data plane for stream-oriented hosts
- pump/drain API for stricter in-process or C ABI hosts

## 4. Control lane и data lane нельзя смешивать в одном callback surface

Host-facing events надо разделить минимум на:

- control/event envelopes
- latest-state updates
- hot binary screen deltas / replay chunks
- terminal-generated replies or write-back requests

Иначе один callback suddenly отвечает и за:

- session exited
- cwd changed
- screen diff arrived
- OSC reply generated
- overflow/resync required

Это плохой API даже до того, как он успеет стать FFI API.

## 5. Node adapter должен мапить onto the contract, not replace it

`napi-rs` слой может быть очень ergonomic:

- `EventEmitter`
- async iterator
- promises for command/reply
- JS objects for snapshots and deltas

Но он не должен быть единственным местом, где определены:

- replay semantics
- overflow/resync semantics
- subscription lifecycle
- object destruction rules

Все эти вещи должны быть описаны на более низком host-neutral уровне.

## Recommended boundary shape for this project

Если проект реально целится в "terminal platform for many hosts", я бы сейчас держал такой shape:

```text
Rust domain/runtime truth
        |
        v
versioned host-neutral protocol + typed projections
        |
        +--> local daemon / socket transport
        +--> C ABI adapter
        +--> Node/Electron adapter
        +--> future generated SDK adapters
```

### Core promises

Stable promises worth making:

- opaque handle lifecycle
- session/pane/subscription identity semantics
- event envelope categories
- stream vs pump semantics
- destroy/drop/close semantics
- overflow/resync semantics
- snapshot/delta schema versioning

### Things to keep out of the promise

- raw emulator grid refs
- Rust memory layout
- binding-generator internals
- Node object identity quirks
- callback scheduling details specific to one host

## Practical package shape

```text
terminal-runtime/
  crates/
    terminal-core/
    terminal-protocol/
    terminal-daemon/
    terminal-capi/
    terminal-node/
    terminal-sdk-shared/
```

Where:

- `terminal-core` owns runtime truth
- `terminal-protocol` owns public envelopes, handles, projections
- `terminal-daemon` exposes local stream-oriented boundary
- `terminal-capi` exposes pump-oriented or callback-sugar C boundary
- `terminal-node` maps the same contract into `napi-rs`
- `terminal-sdk-shared` can later hold adapter-neutral helpers or generated-model glue

## What I would do in v1

1. Make the daemon/protocol contract the primary truth.
2. Make public handles opaque and stable.
3. Expose:
   - commands/queries
   - event drain or stream subscribe
   - snapshot/delta retrieval
   - explicit close/drop functions
4. Build Node/Electron adapter on `napi-rs`.
5. Add deliberate C ABI on `safer-ffi` + `cbindgen`.
6. Evaluate `Diplomat` later as a generated multi-language SDK layer.

## What I would avoid in v1

- ❌ Building the whole public model around foreign callbacks
- ❌ Exposing raw pointers as the normal host API
- ❌ Letting Node promises/events define product semantics
- ❌ Treating generator macros as the product boundary
- ❌ Mixing control events and hot deltas in one accidental callback bus

## Final take

Если сжать всё в одно правило:

🔥 **world-class embeddable terminal package should be runtime-first, protocol-first and handle-first - not binding-first**

А значит:

- `safer-ffi` is the best C ABI path
- `napi-rs` is the best Node/Electron leaf
- `Diplomat` is the most strategically interesting future SDK generator
- `UniFFI` is still useful, but not the architectural center
- callbacks should be convenience adapters over explicit event pumping/streaming, not the core truth model
