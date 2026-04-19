# Deep Dive - Rust Artifact Topology and Multi-Host Distribution

**Проверено**: 2026-04-19

## Зачем этот слой смотреть отдельно

Для terminal runtime мирового уровня мало хорошо разрезать:

- domain
- application
- infrastructure
- host adapters

Нужно ещё заранее решить:

- какие artifact families вообще существуют
- что является source of truth для embedders
- что публикуется как Rust crate
- что публикуется как binary runtime
- что публикуется как C ABI package
- что публикуется как Node/Electron leaf
- какие инструменты отвечают за release orchestration, а какие только за удобную доставку

🔥 Именно здесь многие сильные Rust проекты делают архитектурную ошибку:

- `.node` addon становится фактическим product truth
- `cdylib` живёт отдельно от daemon semantics
- headers и `pkg-config` появляются "потом"
- cross-build tooling случайно диктует crate layout
- installer convenience путают с public compatibility contract

Для universal embeddable terminal package это слишком опасно.

## Primary Sources

### Shipping and artifact orchestration

- [`cargo-dist` crate](https://crates.io/crates/cargo-dist)
- [`cargo-dist` repo](https://github.com/axodotdev/cargo-dist)
- [`cargo-zigbuild` crate](https://crates.io/crates/cargo-zigbuild)
- [`cargo-zigbuild` repo](https://github.com/rust-cross/cargo-zigbuild)
- [`cargo-xwin` crate](https://crates.io/crates/cargo-xwin)
- [`cargo-xwin` repo](https://github.com/rust-cross/cargo-xwin)
- [`cargo-binstall` crate](https://crates.io/crates/cargo-binstall)
- [`cargo-binstall` repo](https://github.com/cargo-bins/cargo-binstall)

### Public ABI and host leaves

- [`safer-ffi` crate](https://crates.io/crates/safer-ffi)
- [`safer-ffi` repo](https://github.com/getditto/safer_ffi)
- [`cbindgen` crate](https://crates.io/crates/cbindgen)
- [`cbindgen` repo](https://github.com/mozilla/cbindgen)
- [`napi` crate](https://crates.io/crates/napi)
- [`napi-rs` repo](https://github.com/napi-rs/napi-rs)
- [`rustls-ffi` repo](https://github.com/rustls/rustls-ffi)

### Donor and future-facing packaging references

- [`maturin` crate](https://crates.io/crates/maturin)
- [`maturin` repo](https://github.com/PyO3/maturin)
- [`cargo-component` crate](https://crates.io/crates/cargo-component)
- [`cargo-component` repo](https://github.com/bytecodealliance/cargo-component)
- [`cxx` repo](https://github.com/dtolnay/cxx)

## Freshness signals

- `cargo-dist 0.31.0` - repo `axodotdev/cargo-dist`, `2006` stars, pushed `2026-04-17`
- `cargo-c 0.10.21+cargo-0.95.0` - repo `lu-zero/cargo-c`, `540` stars, pushed `2026-04-19`
- `cargo-zigbuild 0.22.2` - repo `rust-cross/cargo-zigbuild`, `2430` stars, pushed `2026-04-17`
- `cargo-xwin 0.21.5` - repo `rust-cross/cargo-xwin`, `568` stars, pushed `2026-04-14`
- `cargo-binstall 1.18.1` - repo `cargo-bins/cargo-binstall`, `2615` stars, pushed `2026-04-18`
- `napi 3.8.5` - repo `napi-rs/napi-rs`, `7683` stars, pushed `2026-04-19`
- `safer-ffi 0.2.0-rc1` - repo `getditto/safer_ffi`, `1032` stars, pushed `2026-04-17`
- `cbindgen 0.29.2` - repo `mozilla/cbindgen`, `2870` stars, pushed `2026-04-01`
- `maturin 1.13.1` - repo `PyO3/maturin`, `5557` stars, pushed `2026-04-19`
- `cargo-component 0.21.1` - repo `bytecodealliance/cargo-component`, `585` stars, pushed `2025-07-14`
- `rustls/rustls-ffi` - `163` stars, pushed `2026-04-15`
- `dtolnay/cxx` - `6707` stars, pushed `2026-03-27`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**artifact topology is part of architecture, not release afterthought**

Healthiest shape сейчас выглядит так:

1. core truth lives in Rust crates and versioned protocol contracts
2. daemon/runtime binary is one product surface, not the only one
3. C ABI is a first-class public surface with headers and install story
4. Node/Electron is a leaf adapter above the same truth
5. release tooling orchestrates artifacts, but does not define semantics
6. consumer install convenience must not be confused with compatibility guarantees

То есть не:

- "соберём `.node` и потом посмотрим"

и не:

- "одна `cdylib` magically покроет все host-ы"

а:

- protocol-first workspace
- multiple explicit artifact families
- release matrix as documented product surface
- thin host leaves above shared runtime truth

## Top 3 directions for artifact topology

### 1. `Protocol-first multi-artifact product`

`🎯 10   🛡️ 9   🧠 8`
Примерно `9000-18000` строк.

Это strongest default.

Идея:

- core semantics live in Rust crates and protocol types
- daemon binary is a reusable runtime surface
- C ABI package is separate and serious
- Node/Electron package is a leaf
- release system ships multiple artifact families from one workspace

Почему это лучший путь:

- keeps one architecture across many hosts
- lets Electron, standalone app and foreign-language embedders share the same truth
- makes packaging explicit instead of accidental
- preserves clean boundaries between runtime, ABI, and host UX

### 2. `C ABI-first library + secondary bindings`

`🎯 8   🛡️ 8   🧠 7`
Примерно `7000-15000` строк.

Идея:

- C ABI is the main public embedding surface
- Node and other languages bind to that surface
- daemon story exists, but as a secondary mode

Почему это sometimes works:

- very portable in principle
- easy to explain to many ecosystems
- good if most embedders really want in-process native calls

Почему это weaker than option 1:

- daemon/reconnect/session-runtime story becomes more awkward
- long-lived runtime isolation gets less natural
- it is easier to overfit architecture around ABI mechanics

### 3. `Binding-first per-host packages`

`🎯 3   🛡️ 4   🧠 5`
Примерно `4000-10000` строк на быстрый старт и потом дорого чинить.

Это плохой путь.

Симптомы:

- Node package defines semantics for everyone
- C headers appear later and drift
- CLI/daemon behavior differs from embedded behavior
- release matrix is undocumented tribal knowledge

## 1. One package should expose multiple artifact families, not one magical universal binary

Для такого terminal package важны минимум четыре разные families:

1. Rust crates
2. runtime binaries
3. ABI packages
4. host-specific adapter packages

Healthy split:

- `crates.io` libraries for reusable Rust consumers and internal layering
- standalone daemon/CLI binaries for local runtime mode
- `staticlib` / `cdylib` + headers + `pkg-config` for C-family hosts
- Node addon package for Electron and JS hosts

🔥 Strong rule:

**artifact family should match consumer shape**

Не надо заставлять:

- Python or Swift pretend to be Node
- Electron pretend to be C consumer
- standalone desktop app pretend to be in-process embedder

## 2. `cargo-dist` is the release orchestrator, not the product architecture

`cargo-dist 0.31.0` looks stronger than ever as a release orchestrator.

Why it matters:

- artifact matrix becomes explicit
- release generation becomes repeatable
- installers and archives stop being tribal build knowledge

Why it must stay in its place:

- it does not define your public API
- it does not replace C ABI packaging
- it does not replace Node addon packaging
- it does not decide what artifacts should exist

Healthy role:

- final release graph and publishing pipeline
- install/archive/notarization glue where needed
- distribution discipline above already-correct artifact boundaries

Bad role:

- deciding whether the runtime is daemon-first or binding-first
- hiding missing artifact contracts behind release automation

## 3. `cargo-c` + `safer-ffi` + `cbindgen` is still the strongest C ABI shipping path

This stack remains the most mature serious path for a C-family public surface:

- `safer-ffi` models FFI carriers and ownership cleanly
- `cbindgen` gives real headers
- `cargo-c` gives installable library packaging

Why this trio matters together:

- without `safer-ffi`, ABI design gets sloppy
- without `cbindgen`, headers drift or become handwritten debt
- without `cargo-c`, install story stays ad hoc

`rustls-ffi` remains the best concrete reference I found for treating C ABI as a real product surface.

🔥 Strong rule:

**if we publish a C ABI, it must ship like a real systems library**

That means:

- headers
- consistent symbol surface
- install rules
- `pkg-config` metadata
- documented supported artifact matrix

not just:

- "here is a `.so` or `.dylib`, good luck"

## 4. `napi-rs` should stay a Node leaf, even if Electron is the first host

`napi-rs 3.8.5` is still the best Node/Electron leaf.

Why it is strong:

- very active
- good ergonomics
- mature Node-API story
- good thread-affinity and async adapter support

But the packaging lesson is the same:

- Node addon is one leaf surface
- it must wrap the same runtime/protocol/handle model
- it must not become the real architecture

Healthy role:

- expose host-friendly JS objects and promises
- wrap subscription/event streams
- ship prebuilt `.node` artifacts for Electron consumers

Unhealthy role:

- invent separate semantics unavailable to other hosts
- become the place where session truth actually lives

## 5. `cargo-zigbuild` and `cargo-xwin` solve different shipping jobs

This split matters more than many teams admit.

### `cargo-zigbuild 0.22.2`

Best role:

- portability tuning for Unix-ish targets
- glibc/min-target control
- linker portability improvements

### `cargo-xwin 0.21.5`

Best role:

- Windows MSVC cross-builds
- explicit Windows artifact lane

🔥 Practical rule:

**do not collapse all cross-build concerns into one tool**

Healthy interpretation:

- `cargo-zigbuild` for portability tuning
- `cargo-xwin` for Windows MSVC lane
- maybe native builds where cross-build complexity is not worth it

This is another sign that artifact topology is real architecture:

- your published matrix affects crate layout
- your matrix affects CI
- your matrix affects support promises

## 6. `cargo-binstall` is consumer convenience, not release truth

`cargo-binstall 1.18.1` is useful, but its role is narrow.

What it is good at:

- easier installation for Rust users
- consumption convenience for already-published binaries

What it does **not** solve:

- which artifacts you should publish
- cross-language embed story
- ABI compatibility contracts
- Node or C packaging

Healthy role:

- extra convenience for Rust-native consumers
- nice add-on once binary artifacts already exist

Bad role:

- substitute for documented release matrix
- excuse for not building proper install surfaces

## 7. `maturin` is a strong donor for shipping discipline, but not our architecture center

`maturin 1.13.1` is genuinely useful as a donor because it proves something important:

- one Rust workspace can serve libraries, bindings and binary packaging coherently
- cross-build concerns need explicit tooling
- host-specific packaging conventions matter

But it is still Python-shaped.

That makes it valuable as:

- packaging reference
- multi-artifact discipline donor
- proof that host packaging can be kept leaf-specific

Not valuable as:

- the packaging worldview of our terminal project

## 8. `cargo-component` is interesting future surface, not primary host shipping path

`cargo-component 0.21.1` is still interesting because it reinforces a future-facing idea:

- componentized artifact surfaces may matter later
- Wasm distribution can be a real extra product surface

But for this package today it should remain:

- future sandbox/plugin/export seam
- not primary local runtime or host-embedding story

Why:

- terminal runtime is PTY-heavy
- local session lifecycle matters more than component packaging today
- component-model complexity is real

## 9. `cxx` is a language-specific interop success, but not a universal distribution model

`cxx` remains a very strong project.

Its value here is mostly as a reminder:

- language-specific interop can be excellent
- that still does not make it a universal host contract

So the packaging lesson is:

- C++ can deserve a dedicated leaf one day
- that leaf should still sit above the same runtime truth

Not:

- let each host ecosystem define its own runtime semantics

## 10. The best reference remains `rustls-ffi`

`rustls-ffi` still gives the strongest practical packaging lesson:

- real C ABI
- real headers
- real install surface
- real release discipline

It proves a reusable Rust core can be exported seriously without collapsing architecture around a single foreign host.

🔥 If we want a world-class reusable terminal package, the external seriousness of `rustls-ffi` is a much better packaging reference than any "we ship one addon and call it SDK" story.

## Recommended artifact matrix for this terminal package

### Core publishable crates

- `terminal-domain`
- `terminal-protocol`
- `terminal-runtime`
- `terminal-projections`
- `terminal-testing`

### Infrastructure leaves

- `terminal-pty-portable`
- `terminal-emulator-alacritty`
- optional `terminal-remote-ssh`
- optional `terminal-store-sqlite`

### Public product surfaces

- `terminal-daemon` binary
- `terminal-cli` binary
- `terminal-capi` crate and installable C package
- `terminal-node` package for Node/Electron

### Optional future leaves

- `terminal-component` or sandbox/plugin surface
- additional language SDK generators

Healthy rule:

- one workspace
- one architecture
- many deliberate product surfaces

## Practical stack for shipping this cleanly

### Strong default

- `cargo-dist` - release orchestration
- `cargo-c` - C ABI packaging
- `safer-ffi` - C ABI carriers and ownership model
- `cbindgen` - headers
- `napi-rs` - Node/Electron leaf
- `cargo-zigbuild` - Unix portability tuning
- `cargo-xwin` - Windows cross-build lane

### Useful but secondary

- `cargo-binstall` - Rust-user convenience install
- `maturin` - donor for multi-artifact shipping discipline
- `cargo-component` - future componentized surface

## Things to avoid

- ❌ One host package becoming the de facto architecture
- ❌ Treating release automation as substitute for public contracts
- ❌ Shipping C ABI without headers and install metadata
- ❌ Forcing every host to consume the same artifact type
- ❌ Hiding platform matrix promises inside CI yaml only
- ❌ Letting cross-build tooling dictate domain boundaries

## Final verdict

🔥 For this terminal package, the healthiest artifact strategy is:

- protocol-first core
- multiple explicit artifact families
- daemon binary as one serious surface
- C ABI as another serious surface
- Node/Electron as a thin leaf
- release tooling above that, not instead of that

If we do this right, Electron can be the first consumer without poisoning the package into a Node-shaped runtime.

## Sources

- [cargo-dist](https://github.com/axodotdev/cargo-dist)
- [cargo-c](https://github.com/lu-zero/cargo-c)
- [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild)
- [cargo-xwin](https://github.com/rust-cross/cargo-xwin)
- [cargo-binstall](https://github.com/cargo-bins/cargo-binstall)
- [napi-rs](https://github.com/napi-rs/napi-rs)
- [safer-ffi](https://github.com/getditto/safer_ffi)
- [cbindgen](https://github.com/mozilla/cbindgen)
- [rustls-ffi](https://github.com/rustls/rustls-ffi)
- [maturin](https://github.com/PyO3/maturin)
- [cargo-component](https://github.com/bytecodealliance/cargo-component)
- [cxx](https://github.com/dtolnay/cxx)
