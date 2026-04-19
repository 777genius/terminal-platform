# Deep Dive - Rust Embed Boundaries And Multi-Language Packaging

**Проверено**: 2026-04-19

## Зачем этот deep dive

После Rust runtime ресёрча стало видно, что для отдельного terminal project мало выбрать:

- хороший PTY crate
- хороший emulator core
- нормальный session runtime

Если этот проект должен стать **универсальным modern embeddable package**, который можно использовать:

- внутри Electron app
- как standalone desktop app
- из другой native app
- из другой language runtime

то критичен ещё один слой:

🔥 **что считается public contract этого пакета**

Именно здесь обычно ломаются reusable Rust platforms:

- public API случайно становится набором Node-specific методов
- FFI слой превращается в "настоящую архитектуру"
- Rust ABI или proc-macro binding generator начинают играть роль source of truth
- packaging/install story остаётся ad-hoc и непригодной для внешних команд

## Primary Sources

- [`mozilla/uniffi-rs` README](https://github.com/mozilla/uniffi-rs/blob/main/README.md)
- [`mozilla/cbindgen` README](https://github.com/mozilla/cbindgen/blob/main/README.md)
- [`getditto/safer_ffi` README](https://github.com/getditto/safer_ffi/blob/master/README.md)
- [`napi-rs/napi-rs` `crates/napi/README.md`](https://github.com/napi-rs/napi-rs/blob/main/crates/napi/README.md)
- [`rust-diplomat/diplomat` README](https://github.com/rust-diplomat/diplomat/blob/main/README.md)
- [`ralfbiedert/interoptopus` README](https://github.com/ralfbiedert/interoptopus/blob/master/README.md)
- [`rodrimati1992/abi_stable_crates` readme](https://github.com/rodrimati1992/abi_stable_crates/blob/master/readme.md)
- [`rustls/rustls-ffi` README](https://github.com/rustls/rustls-ffi/blob/main/README.md)

## Короткий вывод

🔥 Для world-class reusable terminal package **не стоит делать binding technology источником истины**.

Лучший shape сейчас выглядит так:

1. **pure Rust runtime core**
2. **versioned host-neutral protocol/control model**
3. **optional C ABI adapter**
4. **host-specific adapters** вроде `napi-rs`, UniFFI или Diplomat поверх той же модели

Иначе каждая интеграция начинает определять продуктовую семантику по-своему.

## Top 3 Public Boundary Strategies

### 1. `Protocol-first runtime + optional C ABI + thin host adapters`

`🎯 10   🛡️ 9   🧠 8`  
Примерно `8000-16000` строк до сильной reusable основы.

Что это значит:

- core runtime живёт в Rust
- наружу экспортируется **versioned control surface**
- базовый внешний contract описывает:
  - session lifecycle
  - command/query/event model
  - streaming/replay semantics
  - capability negotiation
- in-process embedding и non-Rust hosts получают либо:
  - local socket/daemon protocol
  - либо тонкий C ABI слой

Почему это strongest path:

- одинаково хорошо ложится на Electron, Tauri, native host и external automation
- не делает Node/Electron канонической средой
- позволяет UI жить отдельно и даже отдельно перезапускаться
- проще выдерживать DDD/Ports-and-Adapters, потому что contract не спрятан в binding macros

Где риск:

- проектировать protocol и runtime model сложнее, чем просто "экспортировать методы"
- нужно явно держать versioning и capability negotiation

Практический вывод:

✅ Это мой главный recommendation для вашей цели.

### 2. `C ABI-first library + generated headers + thin host adapters`

`🎯 8   🛡️ 8   🧠 7`  
Примерно `6000-12000` строк.

Что это значит:

- Rust runtime собирается как `cdylib` / `staticlib`
- наружу даётся deliberate C ABI
- headers и packaging генерируются и устанавливаются как у нормальной системной библиотеки
- Node/Electron, Swift, Python, C++ и другие хосты живут как adapters above C ABI

Почему это сильно:

- C ABI до сих пор самый универсальный межъязыковой embed boundary
- packaging понятен для многих экосистем
- library можно использовать и in-process, и как часть standalone app

Где риск:

- callback ownership, async model и object lifecycle быстро становятся сложными
- слишком легко превратить низкоуровневый C surface в единственный business API
- удобство для JS/TS хуже, чем у host-native adapters

Практический вывод:

✅ Очень сильный secondary public boundary.  
⚠️ Я бы не делал его единственным source of truth.

### 3. `Binding-generator-first` (`UniFFI` / `Diplomat` / `Interoptopus`)

`🎯 6   🛡️ 6   🧠 5`  
Примерно `4000-9000` строк для первой интеграции.

Что это значит:

- Rust APIs описываются через object model/macros
- генераторы bindings выпускают host-specific interfaces
- разные languages получают "свой" удобный surface

Почему это заманчиво:

- быстрый старт
- меньше ручного FFI glue
- удобно для пары конкретных языков

Почему это не лучший primary contract:

- универсальность зависит от maturity конкретного генератора
- Node/Electron story не у всех сильная
- при долгой жизни продукта business contract начинает быть завязан на ограничения generator-а

Практический вывод:

⚠️ Хорошо как adapter strategy.  
⚠️ Слабо как единственный фундамент для world-class universal package.

## Tool-by-tool findings

## 1. `napi-rs` - excellent Node/Electron adapter, not the public truth

- Crate: [`napi`](https://crates.io/crates/napi)
- Latest: `3.8.5`
- Updated: `2026-04-15`
- Downloads: `30.2M+`
- Repo: [`napi-rs/napi-rs`](https://github.com/napi-rs/napi-rs)

Что видно из primary source:

- это framework for building compiled Node add-ons on top of Node-API
- README требует `crate-type = ["cdylib"]`
- есть `@napi-rs/cli` для packaging/build flow

Почему это важно:

✅ Для Electron adapter story это почти идеальный слой.

Почему не надо делать его core contract:

- он обслуживает Node world, а не "любую app на любом языке"
- если бизнес-семантика terminal runtime живёт прямо в `#[napi]` API, пакет перестаёт быть truly universal

Итог:

✅ Брать как `host_adapter/node`.  
❌ Не делать главным архитектурным boundary.

## 2. `UniFFI` - production-grade multi-language generator, but not universal enough for this role

- Crate: [`uniffi`](https://crates.io/crates/uniffi)
- Latest: `0.31.1`
- Updated: `2026-04-13`
- Downloads: `6.4M+`
- Repo: [`mozilla/uniffi-rs`](https://github.com/mozilla/uniffi-rs)

Что видно из README:

- позиционируется как multi-language bindings generator for Rust
- официально поддерживает `Kotlin`, `Swift`, `Python`, `Ruby`
- third-party bindings listed for `Go`, `C#`, `Dart`, `Java`, `Node`
- Mozilla прямо пишет: production-ready, but still far from `1.0`, advanced things may break

Почему полезен:

- очень сильный tool, если реально нужны mobile/desktop language bindings
- хорош как adapter generator поверх уже сформированного contract

Почему не идеален как primary public boundary:

- Node bindings у него не core story
- продуктовый contract начинает зависеть от UniFFI object model
- для terminal runtime с long-lived sessions, streaming и rich control plane generator может оказаться тесным

Итог:

✅ Хороший optional adapter path.  
⚠️ Не лучший primary truth для reusable terminal platform.

## 3. `Diplomat` - more strategically relevant than I expected

- Crate: [`diplomat`](https://crates.io/crates/diplomat)
- Latest: `0.15.0`
- Updated: `2026-03-31`
- Downloads: `1.84M+`
- Repo: [`rust-diplomat/diplomat`](https://github.com/rust-diplomat/diplomat)

Что видно из README:

- генерирует FFI definitions so that many languages can call Rust code
- прямо перечисляет targets:
  - `C`
  - `C++`
  - `Dart`
  - `Javascript/Typescript`
  - `Kotlin (using JNA)`
  - `Python`

Почему это сильнее, чем кажется:

- ближе к truly multi-language story, чем многие Rust binding generators
- может оказаться полезным, если нужен один declarative adapter layer для нескольких языков сразу

Где риск:

- это всё равно generator layer
- у сложного runtime с streaming/session/process semantics primary truth всё ещё лучше держать отдельно

Итог:

✅ Один из самых интересных adapter-generator paths.  
⚠️ Но всё ещё adapter layer, не product truth.

## 4. `Interoptopus` - strong service/callback model, uneven ecosystem maturity

- Crate: [`interoptopus`](https://crates.io/crates/interoptopus)
- Latest: `0.16.0-alpha.20`
- Updated: `2026-04-14`
- Downloads: `272k+`
- Repo: [`ralfbiedert/interoptopus`](https://github.com/ralfbiedert/interoptopus)

Что видно из README:

- позиционируется как productive, performant, robust interop for Rust
- explicitly supports:
  - structs
  - data-enums
  - callbacks
  - services
  - async
  - idiomatic error handling
- language support is uneven:
  - `C#` tier 1
  - `C` / `Python` tier 2

Почему это полезно:

- ideas around services/callbacks/async are architecturally very relevant for terminal runtime
- может быть сильным donor, если нужен richer object/service boundary than plain C headers

Где риск:

- alpha maturity
- ecosystem coverage пока неравномерная

Итог:

✅ Сильный R&D / donor candidate.  
⚠️ Пока не мой default для universal production package.

## 5. `cbindgen` - header generation only, but still essential

- Crate: [`cbindgen`](https://crates.io/crates/cbindgen)
- Latest: `0.29.2`
- Updated: `2025-10-21`
- Downloads: `81.1M+`
- Repo: [`mozilla/cbindgen`](https://github.com/mozilla/cbindgen)

Что видно из README:

- creates `C/C++11` headers for Rust libraries exposing a public C API
- можно генерировать и C, и C++ headers

Что важно понимать:

⚠️ `cbindgen` **не решает FFI architecture**.  
Он решает генерацию headers.

Это значит:

- ownership model
- async model
- callback safety
- versioning
- package install story

всё равно придётся проектировать отдельно.

Итог:

✅ Essential packaging tool for C ABI layer.  
❌ Не самостоятельная стратегия.

## 6. `safer-ffi` - the cleanest path if we do expose a real C ABI

- Crate: [`safer-ffi`](https://crates.io/crates/safer-ffi)
- Latest: `0.2.0-rc1`
- Updated: `2026-01-16`
- Downloads: `3.1M+`
- Repo: [`getditto/safer_ffi`](https://github.com/getditto/safer_ffi)

Что видно из README:

- helps write FFI without polluting Rust code with `unsafe` blocks
- recommends deliberate crate layout with:
  - `"staticlib"`
  - optional `"cdylib"`
  - `"lib"` for Rust dependents and header generation

Почему это особенно интересно:

- очень хорошо ложится в Clean Architecture adapter crate
- помогает держать Rust code cleaner than ad-hoc manual extern layer

Где риск:

- всё равно нужен disciplined C API design
- не заменяет protocol design и packaging strategy

Итог:

✅ Если делаем C ABI layer, `safer-ffi` сейчас один из лучших кандидатов.

## 7. `abi_stable` - useful, but for a different problem

- Crate: [`abi_stable`](https://crates.io/crates/abi_stable)
- Latest: `0.11.3`
- Updated: `2023-10-12`
- Downloads: `2.7M+`
- Repo: [`rodrimati1992/abi_stable_crates`](https://github.com/rodrimati1992/abi_stable_crates)

Что видно из README:

- designed for `Rust-to-Rust ffi`
- focused on libraries loaded at runtime and load-time type-checking
- good use cases are dynamic Rust libraries and plugin systems

Почему это важно:

⚠️ Это **не** answer to "universal any-language package".

Итог:

✅ Хорошо для Rust plugin/runtime-loaded story.  
❌ Не лучший public boundary для multi-language terminal package.

## 8. `rustls-ffi` + `cargo-c` - the best packaging reference I found

- Repo: [`rustls/rustls-ffi`](https://github.com/rustls/rustls-ffi)
- Packaging tool: [`cargo-c`](https://crates.io/crates/cargo-c)
- `cargo-c` latest: `0.10.21+cargo-0.95.0`
- `cargo-c` updated: `2026-03-07`
- `cargo-c` downloads: `882k+`

Что делает `rustls-ffi` правильно:

- explicitly says "use Rustls from any language"
- documents install flow with `cargo capi install`
- installs:
  - `.a/.so/.dylib/.dll`
  - headers
  - `pkg-config` `.pc` file
- explicitly documents ABI stability caveat instead of pretending it is solved

Почему это очень сильный reference:

- показывает packaging discipline уровня реальной публичной библиотеки
- C ABI treated as product surface, not as build hack

🔥 Если делать terminal runtime как reusable public package, по packaging maturity надо равняться скорее на `rustls-ffi`, а не на "соберём `.node` и как-нибудь подключим".

## 9. `ffi-support` и `async-ffi` - useful tactical helpers only

- [`ffi-support`](https://crates.io/crates/ffi-support) latest `0.4.4`, updated `2021-07-28`
- [`async-ffi`](https://crates.io/crates/async-ffi) latest `0.5.0`, updated `2023-08-11`

Вывод:

- это полезные low-level helpers
- но они не выглядят как правильный центр архитектуры для world-class public package

## Transport choices for the protocol-first layer

Как только architecture becomes protocol-first, появляется следующий вопрос:

**чем именно Rust runtime должен говорить с host-ом**

Здесь полезно сразу разделять:

- local embedding transport
- remote/automation transport
- Rust-to-Rust internal service ergonomics

### 1. `interprocess + own framed protocol`

`🎯 9   🛡️ 9   🧠 6`  
Примерно `3000-7000` строк для сильного local control plane.

- Crate: [`interprocess`](https://crates.io/crates/interprocess)
- Latest: `2.4.1`
- Updated: `2026-04-18`
- Downloads: `8.39M+`
- Repo: [`kotauskas/interprocess`](https://github.com/kotauskas/interprocess)

Что видно из README:

- crate is an IPC toolkit exposing as many platform-specific features as possible while keeping a uniform interface
- gives cross-platform local sockets
- on Windows this maps to named pipes, on Unix to Unix domain sockets
- Tokio async support exists

Почему это самый здоровый default:

- идеально подходит для local daemon boundary
- не навязывает чужую product semantics
- позволяет сделать свой command/query/event protocol exactly under terminal needs
- хорошо ложится на controller/viewer roles, replay streams и capability negotiation

Итог:

✅ Это мой default recommendation для local runtime transport.

### 2. `jsonrpsee`

`🎯 7   🛡️ 8   🧠 6`  
Примерно `3000-8000` строк.

- Crate: [`jsonrpsee`](https://crates.io/crates/jsonrpsee)
- Latest: `0.26.0`
- Updated: `2025-10-22`
- Downloads: `18.8M+`
- Repo: [`paritytech/jsonrpsee`](https://github.com/paritytech/jsonrpsee)

Что видно из README:

- async/await JSON-RPC library for Rust
- supports client/server `HTTP`, `HTTP2`, `WebSocket`
- has client transport abstraction and middleware

Почему это интересно:

- удобен, если сразу хочется formal RPC model
- WebSocket/pubsub story может пригодиться для remote viewers or browser-side tools

Где риск:

- terminal runtime обычно требует не только request/response, но и тонкий streaming/replay contract
- если насильно загонять всё в generic JSON-RPC, можно потерять ясность hot-path semantics

Итог:

✅ Хороший option for external/remote control surfaces.  
⚠️ Для local runtime boundary я бы всё равно сначала предпочёл `interprocess + own protocol`.

### 3. `tonic`

`🎯 6   🛡️ 8   🧠 7`  
Примерно `5000-10000` строк.

- Crate: [`tonic`](https://crates.io/crates/tonic)
- Latest: `0.14.5`
- Updated: `2026-02-19`
- Downloads: `252M+`
- Repo: [`hyperium/tonic`](https://github.com/hyperium/tonic)

Что видно из README:

- gRPC over HTTP/2 focused on performance, interoperability and flexibility
- uses `prost` codegen
- supports bi-directional streaming

Почему это может подойти:

- very interoperable for service-style environments
- good if terminal runtime later becomes remote service consumed by many teams/languages

Где риск:

- for local desktop embedding this is usually too heavy
- protobuf/gRPC discipline adds overhead where a local daemon with framed events might be simpler

Итог:

⚠️ Сильный remote/service option.  
⚠️ Не мой default for local app embedding.

### `tarpc` - interesting, but mainly for Rust-to-Rust internals

- Crate: [`tarpc`](https://crates.io/crates/tarpc)
- Latest: `0.37.0`
- Updated: `2025-08-10`
- Downloads: `7.62M+`
- Repo: [`google/tarpc`](https://github.com/google/tarpc)

Что видно из README:

- code-first RPC framework for Rust
- focuses on ease of use
- schema lives in code, not in a separate IDL

Почему это не top recommendation here:

- хорошо для Rust-to-Rust services
- слабее как universal public boundary for many host languages

Итог:

✅ Useful internal tool.  
❌ Not the primary cross-language contract I would bet on here.

## Recommended package architecture for this project

Если строить **универсальный modern terminal runtime**, я бы целился в такой shape:

```text
terminal-runtime/
  crates/
    terminal-core/
    terminal-protocol/
    terminal-daemon/
    terminal-capi/
    terminal-node/
    terminal-testing/
```

### `terminal-core`

Только domain/runtime:

- PTY/session lifecycle
- emulator integration
- replay/snapshot
- backpressure
- durable scrollback mirror
- command/query/event model

Никакого `napi`, `extern "C"` или Electron knowledge.

### `terminal-protocol`

Host-neutral contract:

- command/query DTOs
- event envelopes
- capability negotiation
- protocol versioning
- error categories

Это должен быть один из главных stable contracts.

### `terminal-daemon`

Optional out-of-process host:

- local socket server
- same protocol as above
- attach/detach/reconnect
- crash isolation from UI

Именно это делает пакет reusable даже там, где in-process embedding неудобен.

### `terminal-capi`

Secondary universal adapter:

- deliberate C ABI
- generated headers
- packaging via `cargo-c`
- possibly implemented with `safer-ffi` and `cbindgen`

### `terminal-node`

Node/Electron adapter:

- built with `napi-rs`
- thin translation onto protocol/core semantics
- no business truth living only here

### `terminal-testing`

Shared harness:

- automation APIs
- snapshot helpers
- protocol conformance tests

## Stable promises we should make

Если это реальный reusable package, стабильность надо обещать не абстрактно, а по слоям:

- protocol versioning rules
- session lifecycle semantics
- replay/overflow semantics
- ownership and destruction model
- C ABI packaging/install shape
- Node adapter support matrix

⚠️ Не надо обещать "вообще всё стабильно", если реально стабилен только high-level protocol.

## What I would not do

- ❌ Не делать raw Rust ABI публичным contract
- ❌ Не делать `#[napi]` API каноническим business surface
- ❌ Не полагаться только на binding generator для streaming/session semantics
- ❌ Не выпускать public C ABI без headers, install story и `pkg-config` metadata
- ❌ Не смешивать protocol truth и host adapter specifics

## Final take

Если смотреть именно на **универсальный Rust terminal package мирового уровня**, то лучший курс сейчас такой:

- runtime/core держать pure Rust
- public architecture делать **protocol-first**
- C ABI держать как serious secondary boundary
- `napi-rs`, UniFFI, Diplomat и подобные вещи использовать как **thin adapters**

Иначе вместо reusable platform получится "один очень умный Electron addon".
