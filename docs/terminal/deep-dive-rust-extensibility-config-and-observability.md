# Deep Dive - Rust Extensibility, Config Evolution, And Observability Boundaries

**Проверено**: 2026-04-19

## Зачем этот deep dive

Если terminal runtime должен стать universal embeddable package, то мало выбрать:

- PTY layer
- emulator core
- protocol boundary
- release discipline

Нужно ещё решить три вещи, которые потом очень тяжело переделывать:

1. как package расширяется
2. как package конфигурируется и эволюционирует
3. как package наблюдается снаружи

Именно здесь часто появляется архитектурный шум:

- dynamic plugins тащат ABI complexity слишком рано
- config loading становится app-specific и перестаёт быть reusable
- telemetry либо жёстко вшивается, либо вообще не проектируется как public seam

## Primary Sources

- [`inventory` README](https://github.com/dtolnay/inventory/blob/master/README.md)
- [`libloading` README](https://github.com/nagisa/rust_libloading/blob/master/README.mkd)
- [`abi_stable` readme](https://github.com/rodrimati1992/abi_stable_crates/blob/master/readme.md)
- [`stabby` README](https://github.com/ZettaScaleLabs/stabby/blob/main/README.md)
- [`Figment` README](https://github.com/SergioBenitez/Figment/blob/master/README.md)
- [`config-rs` README](https://github.com/rust-cli/config-rs/blob/main/README.md)
- [`serde_path_to_error` README](https://github.com/dtolnay/path-to-error/blob/master/README.md)
- [`arc-swap` README](https://github.com/vorner/arc-swap/blob/master/README.md)
- [`notify` README](https://github.com/notify-rs/notify/blob/main/README.md)
- [`OpenTelemetry Rust` README](https://github.com/open-telemetry/opentelemetry-rust/blob/main/README.md)

## Freshness Signals

На `2026-04-19` сигналы такие:

- `tracing-opentelemetry 0.32.1`, repo `387` stars, push `2026-02-16`
- `opentelemetry 0.31.0`, repo `2548` stars, push `2026-04-18`
- `inventory 0.3.24`, repo `1291` stars, push `2026-03-30`
- `libloading 0.9.0`, repo `1438` stars, push `2026-01-07`
- `abi_stable 0.11.3`, repo `591` stars, last visible push `2023-10-12`
- `stabby 72.1.2-rc1`, repo `417` stars, push `2026-02-23`
- `figment 0.10.19`, repo `889` stars, push `2024-09-13`
- `config 0.15.22`, repo `3134` stars, push `2026-04-16`
- `serde_path_to_error 0.1.20`, repo `422` stars, push `2026-02-16`
- `arc-swap 1.9.1`, repo `1313` stars, push `2026-04-04`
- `notify 9.0.0-rc.3`, repo `3333` stars, push `2026-04-19`

## Короткий вывод

🔥 Для terminal runtime v1 я бы проектировал extensibility и operations так:

1. **core extensibility through typed protocol + feature-gated adapters**
2. **config model through `serde + schemars + path-aware diagnostics`**
3. **observability through `tracing`, with optional OpenTelemetry bridge**
4. **no dynamic Rust plugin ABI as the default extension path**

## Top 3 Extensibility Strategies

### 1. `Static built-ins + protocol-level extensibility + optional sidecars`

`🎯 10   🛡️ 9   🧠 7`  
Примерно `5000-11000` строк.

Что это значит:

- core runtime ships with explicit ports/adapters
- host apps extend behavior through:
  - config
  - protocol commands/events
  - external sidecars/services
  - feature-gated internal adapters
- in-process extensions stay compile-time and explicit

Почему это strongest path:

- no unstable dynamic ABI burden in v1
- easier to keep cross-language embedding clean
- keeps extension seam above the runtime core, not inside memory layout tricks

Практический shape:

- `inventory` only for compile-time registries if needed
- typed protocol for out-of-process extensions
- `serde + schemars` config schema
- `tracing` events as observability seam

### 2. `Static registries + hot-reloadable config + host-controlled operations`

`🎯 8   🛡️ 8   🧠 6`  
Примерно `4000-9000` строк.

Что это значит:

- no plugin ABI
- richer host-controlled behavior through config overlays and runtime toggles
- optional file watching and atomic config snapshot swap

Почему это интересно:

- many "plugin" use cases are actually config + policy changes
- easier to secure, test and document

Где риск:

- can become over-configured if capabilities are not modeled cleanly
- watch/reload semantics become a real domain seam

### 3. `Dynamic in-process plugin ABI`

`🎯 4   🛡️ 5   🧠 10`  
Примерно `9000-18000` строк.

Что это значит:

- runtime loads dynamic libraries
- plugin ABI needs versioning, type checks and lifecycle contracts
- host and plugin toolchains become part of compatibility surface

Почему это risky:

- cross-language story gets worse, not better
- dynamic ABI and plugin unloading/lifetime rules are hard
- terminal runtime already has enough lifecycle complexity without adding unstable plugin semantics too early

Практический вывод:

⚠️ Interesting future seam.  
❌ I would not make it the default v1 extension model.

## Library-by-library findings

## 1. `inventory` - strong for compile-time registries, not a runtime plugin system

- `inventory 0.3.24`
- typed distributed plugin registration
- registration from any source file linked into the application
- life-before-main style registration
- dynamically loaded libraries register entries at `dlopen` time

Почему это полезно:

- built-in commands
- built-in providers
- compile-time registries across workspace crates

Почему это не надо путать с true plugin architecture:

- it is still linked-code registration
- it does not solve independent versioned third-party plugin compatibility

Итог:

✅ Great for static extension points inside the workspace.  
❌ Not the main answer for external plugin ABI.

## 2. `libloading` - necessary low-level primitive, not a plugin architecture

- `libloading 0.9.0`
- safe wrapper around platform dynamic library loading
- strongest guarantee mentioned is preventing dangling `Symbol`s after unload

Почему это важно:

- if dynamic libs ever appear, `libloading` is the basic primitive

Почему этого недостаточно:

- loading is not compatibility
- symbol lookup is not semantic versioning
- it does not define type layout contracts or extension lifecycle

Итог:

✅ Keep as a low-level tool only if dynamic loading is truly needed.  
❌ Never let it define the architecture by itself.

## 3. `abi_stable` - still the clearest Rust-to-Rust plugin story, but not ideal for v1

- `abi_stable 0.11.3`
- explicitly for Rust-to-Rust FFI
- load-time type checking
- prefix types for extensible modules and vtables
- ffi-safe trait objects and nonexhaustive enums
- explicitly mentions plugin systems without unloading

Почему это серьёзно:

- this is the most mature Rust-side answer to "how do I version dynamic Rust plugins at all?"
- prefix/module model is genuinely relevant to long-lived extension contracts

Почему всё равно не default:

- repo activity is much quieter than core runtime crates
- Rust-to-Rust ABI is still a niche compared to host-neutral protocol boundaries
- third-party plugin compatibility becomes a whole product on its own

Итог:

✅ Strong future option if you later commit to Rust plugin ecosystem.  
⚠️ Too much complexity for default v1 extensibility.

## 4. `stabby` - ambitious, active, but too sharp for the mainline

- `stabby 72.1.2-rc1`
- stable ABI ambitions for Rust
- canaries and type reports
- import/export helpers
- support for futures over ABI-safe trait objects
- SemVer policy explicitly includes ABI dimension

Почему это очень интересно:

- the project thinks directly in ABI/versioned plugin terms
- canary model is more serious than most ad-hoc FFI wrappers

Почему я бы не ставил это в основу:

- current release is still `rc`
- README explicitly warns about trait-object performance issues after Rust 1.78
- it is a deep commitment to a very specific ABI strategy

Итог:

🔥 Excellent research signal.  
⚠️ Not the conservative foundation for a universal terminal platform.

## 5. `config-rs` vs `Figment`

### `config-rs`

- `config 0.15.22`
- layered configuration system
- strong 12-factor positioning
- many formats
- env support
- custom format extensibility

### `Figment`

- `figment 0.10.19`
- semi-hierarchical configuration library
- elegant merge/join model
- providers for env/TOML/JSON and more

Главный вывод:

- `config-rs` feels like the stronger general-purpose app loader today
- `Figment` is elegant, but activity signal is weaker

Но для terminal package core я бы всё равно не делал ни один из них "истиной".

Правильнее:

- public config structs with `serde`
- schema/export with `schemars`
- host/application chooses loader strategy

Итог:

✅ Use a config loader in app layers or tooling.  
⚠️ Keep core config types independent of a specific loading crate.

## 6. `serde_path_to_error` - extremely useful for host integration quality

- `serde_path_to_error 0.1.20`
- exposes the path that failed during deserialization

Почему это важно:

- host apps need actionable config errors
- “invalid config” is not enough when configs come from Electron, CLI, env files or remote launch requests

Итог:

✅ This is a small crate with very high product value.  
🔥 Strong recommendation for config and manifest ingestion boundaries.

## 7. `arc-swap` - strong primitive for read-mostly runtime snapshots

- `arc-swap 1.9.1`
- like `RwLock<Arc<T>>`, optimized for read-mostly write-seldom cases
- good fit for atomic config snapshot replacement

Почему это полезно:

- hot-reloadable policies
- immutable runtime views
- readers can observe a coherent config snapshot without lock-heavy access patterns

Итог:

✅ Great fit if config/policy reload is needed.  
⚠️ Use for swapping snapshots, not as excuse to hide mutable global state everywhere.

## 8. `notify` - useful, but keep watch mode optional

- `notify 9.0.0-rc.3`
- cross-platform filesystem notification
- widely used by serious tools
- explicit MSRV policy

Почему это полезно:

- development mode config reload
- manifest/theme/profile watchers
- external integration surfaces

Почему осторожно:

- current latest is still `rc`
- file watching semantics differ across platforms
- runtime correctness should not depend on watching working perfectly

Итог:

✅ Good optional operations feature.  
⚠️ Do not make core runtime correctness depend on it.

## 9. `OpenTelemetry` and `tracing-opentelemetry` - observability bridge, not core API

### `opentelemetry`

- `opentelemetry 0.31.0`
- API and SDK ecosystem for traces, metrics and logs
- project status table is explicit about signal stability
- README recommends `tracing` as the logging API when starting fresh

### `tracing-opentelemetry`

- `tracing-opentelemetry 0.32.1`
- integration crate for bridging `tracing` to OpenTelemetry

Главный вывод:

🔥 Core runtime should emit `tracing`.

Then:

- standalone app can install exporters
- server mode can bridge to OTEL
- embedders can keep telemetry entirely local if they want

Итог:

✅ `tracing` remains the core semantic event surface.  
✅ OpenTelemetry should stay an optional bridge layer, not a required dependency of the public contract.

## Самые важные architectural выводы

### 1. Prefer extension above the core, not inside the ABI

Most v1 extension needs are better solved by:

- config
- typed protocol
- external sidecars
- feature-gated adapters

than by dynamic Rust plugin loading.

### 2. Separate config truth from config loading

Core package owns:

- config types
- schema
- validation
- path-aware errors

Host/application owns:

- file formats
- env layering
- watch mode
- CLI integration

### 3. Make observability optional but first-class

Runtime should be richly instrumented, but not force a specific exporter stack on embedders.

### 4. Hot reload should swap immutable snapshots, not mutate live shared state

This is where `arc-swap` becomes more interesting than generic mutex-heavy config managers.

### 5. Dynamic plugin ABI is a separate product decision

If you ever adopt it, it deserves:

- versioned extension contract
- plugin packaging story
- compatibility tests
- strong support policy

That is too much to sneak into v1 "because it sounds universal".

## Current Practical Recommendation

Если выбирать сейчас, я бы делал так:

1. **Core extensibility**
   - feature-gated adapters
   - compile-time registries where useful via `inventory`
   - no dynamic plugin ABI as a default

2. **Config boundary**
   - `serde` + `schemars`
   - `serde_path_to_error`
   - loader crate chosen by app layer, not by core

3. **Optional operations**
   - `arc-swap` for atomic snapshot swapping
   - `notify` only as optional watch/reload helper

4. **Observability**
   - core emits `tracing`
   - optional `tracing-opentelemetry` bridge for hosts that want OTEL

5. **Future-only seam**
   - evaluate `abi_stable` or `stabby` only if a real third-party plugin ecosystem becomes a product goal

## Sources

- [inventory](https://github.com/dtolnay/inventory)
- [libloading](https://github.com/nagisa/rust_libloading)
- [abi_stable](https://github.com/rodrimati1992/abi_stable_crates)
- [stabby](https://github.com/ZettaScaleLabs/stabby)
- [Figment](https://github.com/SergioBenitez/Figment)
- [config-rs](https://github.com/rust-cli/config-rs)
- [serde_path_to_error](https://github.com/dtolnay/path-to-error)
- [arc-swap](https://github.com/vorner/arc-swap)
- [notify](https://github.com/notify-rs/notify)
- [OpenTelemetry Rust](https://github.com/open-telemetry/opentelemetry-rust)
- [tracing-opentelemetry crate](https://crates.io/crates/tracing-opentelemetry)
