# Deep Dive - Rust Sandboxed Extensions, Wasm Plugins, And Sidecar Isolation

**Проверено**: 2026-04-19

## Зачем этот слой важен

Для universal embeddable terminal package почти неизбежно появится соблазн добавить:

- analyzers
- enrichers
- custom command summarizers
- search/index sidecars
- automation hooks
- third-party extensions

Именно тут очень легко смешать три разных вопроса в один:

1. host embedding boundary
2. plugin/extension boundary
3. sandbox/isolation boundary

🔥 Для world-class пакета это критично разделить.

Иначе быстро получится одна из плохих архитектур:

- host SDK случайно становится plugin API
- plugin model случайно становится единственным sandbox story
- runtime начинает таскать в core всю сложность Wasm/framework ecosystem раньше времени

## Primary Sources

### Wasm runtime and component model

- [`wasmtime` README](https://github.com/bytecodealliance/wasmtime/blob/main/README.md)
- [`wasmtime-wasi` docs](https://docs.rs/wasmtime-wasi)
- [`wit-bindgen` README](https://github.com/bytecodealliance/wit-bindgen/blob/main/README.md)
- [`wrpc` README](https://github.com/bytecodealliance/wrpc/blob/main/README.md)

### Extism ecosystem

- [`extism` README](https://github.com/extism/extism/blob/main/README.md)
- [`extism-pdk` README](https://github.com/extism/rust-pdk/blob/main/README.md)

### Alternative lightweight runtime

- [`wasmi` docs](https://docs.rs/wasmi)

## Freshness signals

- `wasmtime 43.0.1` - repo `bytecodealliance/wasmtime`, `17898` stars, pushed `2026-04-17`
- `wasmtime-wasi 43.0.1`
- `wit-bindgen 0.57.1` - repo `bytecodealliance/wit-bindgen`, `1382` stars, pushed `2026-04-17`
- `wrpc 0.16.0` / `wrpc-transport 0.28.4` - repo `bytecodealliance/wrpc`, `320` stars, pushed `2026-04-19`
- `extism 1.21.0` - repo `extism/extism`, `5569` stars, pushed `2026-03-26`
- `extism-pdk 1.4.1` - repo `extism/rust-pdk`, `72` stars, pushed `2026-02-09`
- `wasmi 2.0.0-beta.2` - repo `wasmi-labs/wasmi`, `2100` stars, pushed `2026-03-26`

## Короткий вывод

🔥 Самый здоровый shape сейчас такой:

1. v1 extensibility through typed protocol and built-ins
2. untrusted or experimental logic through process-isolated sidecars
3. if later needed, sandboxed Wasm extensions through `wasmtime + WIT/WASI`
4. plugin framework choices like `Extism` only as a conscious product decision

То есть healthiest architecture is **not**:

- "раз мы уже делаем host-neutral API, значит это и есть plugin API"
- "раз у нас есть Wasm, значит нужно сразу тащить full plugin platform в core"
- "раз есть Extism, значит extension story уже solved"

## Top 3 directions for extension/sandbox architecture

### 1. `Protocol-first built-ins + process-isolated sidecars`

`🎯 10   🛡️ 10   🧠 7`  
Примерно `7000-14000` строк.

Это мой текущий **лучший default**.

Идея:

- core runtime stays focused
- built-in enrichers/analyzers are ordinary leaves behind ports
- untrusted or experimental logic runs out-of-process
- communication goes through the same typed control/data surfaces

Почему это strongest path:

- clearest fault containment
- easiest to reason about lifecycle and memory
- no premature commitment to one plugin ecosystem
- works for Rust, Python, JS or any other sidecar implementation

Это особенно хорошо ложится на уже найденные правила:

- daemon-first runtime
- protocol-first contracts
- explicit capability/security model
- panic/fault containment by boundary

### 2. `Wasmtime + WIT/WASI component model for sandboxed extensions`

`🎯 8   🛡️ 9   🧠 8`  
Примерно `9000-18000` строк.

Идея:

- plugin boundary is separate from host SDK boundary
- extension contract is described in WIT
- guest logic compiles to Wasm components
- runtime hosts them via `wasmtime` / `wasmtime-wasi`

Почему это very strong:

- strongest long-term standards-oriented sandbox path
- explicit interface model
- good multi-language story
- strong security and conformance posture in the upstream ecosystem

Где риск:

- a lot more moving parts than sidecars
- component/WASI story becomes part of your product complexity
- not all extension jobs are worth a full Wasm component model

### 3. `Extism as pragmatic multi-language plugin framework`

`🎯 7   🛡️ 7   🧠 6`  
Примерно `7000-15000` строк.

Идея:

- use Extism host runtime and PDK ecosystem
- let extension authors target a more batteries-included plugin framework
- benefit from existing cross-language SDK/PDK story

Почему это compelling:

- very plugin-oriented from day one
- good multi-language ergonomics
- already thinks in terms of host-controlled capabilities and plugin packaging

Где риск:

- you adopt Extism’s worldview, not just a small runtime crate
- plugin framework semantics can start shaping your product semantics
- not as standards-neutral as a raw WIT/Wasmtime approach

Практический вывод:

✅ Strong pragmatic option if plugins become a true product feature.  
⚠️ Not my default center for the runtime today.

## 1. The most important rule: plugin boundary is not host boundary

This is the biggest architectural lesson of this pass.

Your package needs at least three different boundaries:

- **host embedding boundary**  
  JS/Electron, C ABI, Node adapter, future Swift/Python host SDKs

- **runtime control boundary**  
  sessions, panes, transcripts, projections, commands

- **extension boundary**  
  analyzers, enrichers, optional third-party logic, experiments

🔥 These should not collapse into one API.

If they do:

- plugin lifecycle infects host SDK design
- host ergonomics start constraining sandbox design
- runtime core becomes harder to keep clean

## 2. `wasmtime` is the strongest long-term standards-oriented sandbox brick

`wasmtime` README remains very convincing.

Important signals:

- strong security posture
- active RFC/review process
- 24/7 fuzzing and formal verification work
- official multi-language support
- configurable runtime resource controls

For our use-case the most important thing is not "Wasm is cool".

It is this:

🔥 `wasmtime` gives a credible **separate execution environment** with clear language-independent boundaries.

That makes it the strongest long-term building block if you ever want:

- sandboxed enrichers
- analyzers over transcript chunks
- third-party extension points
- host-neutral extension contracts

But it is still heavy enough that it should justify itself.

## 3. `wasmtime-wasi` makes the sandbox story real, but also imports product complexity

`wasmtime-wasi` is what turns plain Wasm hosting into a more practical execution environment.

Why that matters:

- many meaningful extensions want controlled I/O
- terminal-adjacent enrichers may need files, config, clocks, maybe limited networking

But the architectural warning is important:

⚠️ once you adopt a WASI-backed extension model, capability policy becomes part of your product core

That means:

- what FS authority does a plugin get?
- what network authority?
- does it see transcript history?
- can it emit commands or only metadata?

So `wasmtime-wasi` is strong, but it pulls in a real policy surface.

## 4. `wit-bindgen` is strongest when the extension contract itself matters

`wit-bindgen 0.57.1` is still the cleanest signal for interface-first Wasm extensions.

Its strongest value here:

- interface described separately from implementation
- multi-language guest potential
- a cleaner future than inventing ad-hoc plugin ABIs

This matters because extension contracts for a terminal package are likely to need typed concepts like:

- `TranscriptChunk`
- `CommandSummary`
- `LinkCandidate`
- `StatusUpdate`
- `SuggestionBatch`

🔥 If that contract matters long-term, WIT is a far healthier center than raw ABI or random JSON callbacks.

## 5. `Extism` is the most pragmatic plugin framework, but it is a product choice, not just a crate

The `Extism` README is very explicit:

- primary use case is extensible software and plugins
- it supports secure, host-controlled HTTP without WASI
- it offers runtime limiters, timers, host linking and more
- it has broad host SDK coverage across many languages

This is all real value.

`extism-pdk` also makes guest authoring comparatively approachable:

- simple `#[plugin_fn]`
- typed conversions
- config access
- host/plugin data exchange helpers

That makes `Extism` the most convincing "we want plugins soon" option.

But it also means:

⚠️ You are not just taking a sandbox engine.  
You are adopting a plugin framework worldview.

For a universal terminal runtime that matters because:

- plugin packaging semantics
- host/plugin config semantics
- guest ABI ergonomics
- capability exposure style

can start driving your architecture.

## 6. `wRPC` is extremely interesting, but it is later than it looks

`wRPC` is one of the more interesting fresh signals here.

The README is very clear:

- component-native
- transport-agnostic
- WIT-based
- supports streams and futures
- main use cases include out-of-tree runtime plugins and distributed component communication

This is powerful.

But for our package today, it likely belongs to a later phase.

Why:

- it is more distributed-system-shaped than local runtime shaped
- it adds another abstraction layer over transport
- it is more relevant when extensions are already components and maybe remote/distributed

Practical takeaway:

✅ Very interesting future seam for distributed extension fleets or remote component services.  
⚠️ Too early for the core v1 runtime.

## 7. `wasmi` is an interesting lighter engine, but not the strongest default

`wasmi 2.0.0-beta.2` is attractive because:

- interpreter-oriented
- smaller-feeling than a full Wasmtime stack
- active project with decent ecosystem gravity

But for this package it still feels more like:

- evaluation seam
- lighter experimental host
- specialized environment

than the strongest default center.

Main reason:

- current version line is still beta
- standards/ecosystem gravity is weaker than Wasmtime

Practical takeaway:

✅ Good watchlist item.  
⚠️ Not my first choice for a world-class extension/sandbox story.

## 8. What I would actually build

For this terminal package I would shape extension/sandbox layers like this:

### v1

- no general third-party plugin ABI in core
- built-in enrichers and analyzers behind typed internal ports
- optional out-of-process sidecars for untrusted or experimental logic
- same protocol discipline as the rest of the runtime

### v2 if sandboxed extensions become real

- WIT-described extension contracts
- `wasmtime + wasmtime-wasi` host
- explicit capability policy per extension kind
- extension boundary separate from host SDK boundary

### only if plugin ecosystem becomes a primary product goal

- evaluate `Extism` as a framework decision
- or standardize a component-model-based extension SDK yourselves

## Practical recommendations

- ✅ Keep plugin boundary separate from host embedding boundary
- ✅ Prefer sidecars first for risky or experimental logic
- ✅ If adopting Wasm sandboxing, make capabilities explicit from day one
- ✅ Treat `wasmtime + WIT/WASI` as the strongest long-term standards-oriented path
- ✅ Treat `Extism` as a conscious framework choice, not a tiny helper crate
- ✅ Treat `wRPC` as a future distributed extension seam
- ⚠️ Do not make plugin system a v1 excuse to overcomplicate the runtime core
- ❌ Do not let host SDK API accidentally become third-party plugin API

## Sources

- [wasmtime README](https://github.com/bytecodealliance/wasmtime/blob/main/README.md)
- [wasmtime-wasi](https://docs.rs/wasmtime-wasi)
- [wit-bindgen README](https://github.com/bytecodealliance/wit-bindgen/blob/main/README.md)
- [wRPC README](https://github.com/bytecodealliance/wrpc/blob/main/README.md)
- [Extism README](https://github.com/extism/extism/blob/main/README.md)
- [Extism Rust PDK README](https://github.com/extism/rust-pdk/blob/main/README.md)
- [wasmi](https://docs.rs/wasmi)
