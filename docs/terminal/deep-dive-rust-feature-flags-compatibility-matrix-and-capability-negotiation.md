# Deep Dive - Rust Feature Flags, Compatibility Matrix, And Capability Negotiation

**Проверено**: 2026-04-19

## Зачем этот слой важен

Для world-class embeddable terminal package очень легко сделать одну из двух ошибок:

- считать, что Cargo features и есть вся compatibility story
- или вообще не разделить compile-time and runtime capabilities

Но у такого пакета реально есть как минимум три разных уровня совместимости:

1. **crate/build-time surface**
2. **published package compatibility contract**
3. **runtime capability negotiation**

🔥 Если это смешать, начинаются типичные проблемы:

- giant feature soup
- трудно воспроизводимые host builds
- неочевидные support promises
- runtime, который не умеет честно сказать host-у, какие возможности реально доступны

## Primary Sources

### Core versioning and compatibility crates

- [`semver` docs](https://docs.rs/semver)
- [`target-lexicon` docs](https://docs.rs/target-lexicon)
- [`version_check` docs](https://docs.rs/version_check)

### Feature and cfg tooling

- [`cfg_aliases` docs](https://docs.rs/cfg_aliases)
- [`document-features` docs](https://docs.rs/document-features)

### Matrix/testing/release discipline tools

- [`cargo-hack` docs](https://docs.rs/cargo-hack)
- [`cargo-msrv` docs](https://docs.rs/cargo-msrv)
- [`cargo-minimal-versions` docs](https://docs.rs/cargo-minimal-versions)
- [`cargo-nextest` docs](https://nexte.st)

## Freshness signals

- `semver 1.0.28` - repo `dtolnay/semver`, `662` stars, pushed `2026-04-04`
- `cfg_aliases 0.2.1` - repo `katharostech/cfg_aliases`, `96` stars, pushed `2025-04-16`
- `document-features 0.2.12` - repo `slint-ui/document-features`, `192` stars, pushed `2025-10-24`
- `target-lexicon 0.13.5` - repo `bytecodealliance/target-lexicon`, `822` stars, pushed `2026-04-10`
- `cargo-msrv` latest `0.19.3` with cargo info resolving `0.18.4`, repo `foresterre/cargo-msrv`, `1172` stars, pushed `2026-03-26`
- `cargo-hack 0.6.44` - repo `taiki-e/cargo-hack`, `56` stars surfaced through `gh repo view` resolution, crate active
- `cargo-minimal-versions 0.1.37`
- `cargo-nextest` latest `0.9.133`, cargo info path observed `0.9.128`

## Короткий вывод

🔥 Самый здоровый shape сейчас такой:

1. small stable core crates
2. optional adapter leaves in separate crates when possible
3. minimal Cargo feature surface inside each crate
4. explicit documented capability matrix
5. runtime handshake that reports compiled and available capabilities

То есть healthiest architecture is **not**:

- one mega crate with dozens of default-on features
- hidden cfg logic spread through the codebase
- runtime assuming that compiled == available == authorized

## Top 3 directions for compatibility architecture

### 1. `Small core crates + optional leaves + runtime capability handshake`

`🎯 10   🛡️ 9   🧠 7`  
Примерно `7000-14000` строк.

Это мой текущий **лучший default**.

Идея:

- core runtime crates keep feature flags minimal
- optional concerns move into separate crates when they are big enough:
  - Node adapter
  - C ABI adapter
  - remote SSH route
  - Wasm sandbox host
  - standalone renderer/app shell
- runtime exposes a typed capability handshake at startup/attach time

Почему это strongest path:

- compatibility story becomes understandable
- fewer accidental feature interactions
- semver surface is easier to reason about
- runtime can honestly tell the host what is compiled and what is currently usable

### 2. `One crate with disciplined features + documented matrix + strong CI`

`🎯 8   🛡️ 8   🧠 7`  
Примерно `8000-15000` строк.

Идея:

- keep more functionality in one crate
- use disciplined feature groups
- use `cfg_aliases`, `document-features`, `cargo-hack`, `cargo-msrv`
- maintain a tested feature matrix and explicit docs

Почему это workable:

- sometimes simpler for users initially
- still manageable if feature count stays small and curated

Где риск:

- pressure to keep adding feature flags rises over time
- adapters can start leaking into the core crate
- docs and CI need to stay unusually disciplined

### 3. `Feature-soup crate with runtime guessing`

`🎯 3   🛡️ 4   🧠 5`  
Примерно `5000-12000` строк на старт и потом дорого чинить.

Идея:

- large single crate
- many optional features
- runtime infers support indirectly from build shape or host behavior

Почему это bad path:

- hardest to publish and support
- easiest to break semver and MSRV expectations
- capability negotiation becomes vague
- users cannot tell what is guaranteed versus merely compiled

## 1. Cargo features and runtime capabilities are different kinds of truth

This is the central rule of this pass.

Cargo features answer things like:

- was SSH adapter compiled?
- was WASI sandbox host compiled?
- was bundled SQLite enabled?

Runtime capability negotiation should answer things like:

- is remote route available in this build?
- is it configured for this host/session?
- is this client allowed to use it?
- is clipboard/status/search supported for this attach role?

🔥 A compiled feature is not the same as a runtime capability.

And even a runtime capability is not the same as authority.

This lines up directly with earlier conclusions about:

- capability/security model
- controller/viewer roles
- route/authority metadata

## 2. `semver` should help model explicit compatibility, not just power release notes

`semver 1.0.28` is not flashy, but it matters here because compatibility policy for the package should be machine-readable where possible.

Useful roles:

- interpreting host/plugin/extension version ranges
- validating compatibility between daemon and host SDK versions
- expressing minimum supported protocol or extension schema ranges

Practical takeaway:

✅ use `semver` where compatibility rules are real runtime inputs  
⚠️ do not leave version logic as ad-hoc string comparison

## 3. `cfg_aliases` is good hygiene, not a design strategy

`cfg_aliases 0.2.1` is tiny, but very useful.

Best role:

- centralize messy platform/feature cfg expressions
- keep build.rs and `#[cfg]` usage readable

Why that matters:

- this package will likely have platform leaves
- optional adapters
- optional sandbox/remote layers

Without alias hygiene, cfg expressions spread everywhere and become architecture debt.

But:

⚠️ `cfg_aliases` only reduces build-condition mess.  
It does not solve product compatibility by itself.

## 4. `document-features` is underrated for public package trust

`document-features 0.2.12` is more important than it first looks.

For a package meant to be reused widely, feature docs should not live only in tribal knowledge or scattered markdown.

This crate gives a disciplined path to:

- derive feature documentation from `Cargo.toml`
- keep docs closer to real build surface

Practical takeaway:

🔥 If feature flags exist at all, they should be documented as part of the package contract, not as incidental implementation toggles.

## 5. `target-lexicon` is the cleanest donor for honest target-aware behavior

`target-lexicon 0.13.5` is a good fit when host/runtime behavior differs by:

- OS
- arch
- ABI
- environment

This package likely has genuine target-dependent leaves:

- Unix PTY vs Windows ConPTY
- maybe renderer or sandbox support
- maybe bundled vs system SQLite choices

`target-lexicon` is much healthier than ad-hoc target-triple string parsing.

Practical takeaway:

✅ good internal brick for target-aware capability shaping  
⚠️ keep it inside runtime/build logic, not as a host-facing abstraction center

## 6. `cargo-msrv`, `cargo-hack`, and `cargo-minimal-versions` together describe the real compatibility matrix

This is one of the stronger package-discipline findings of this pass.

### `cargo-msrv`

Use it to validate:

- minimum supported Rust version as policy, not hope

### `cargo-hack`

Use it to validate:

- feature combinations
- no-default-features
- leaf feature isolation

### `cargo-minimal-versions`

Use it to validate:

- dependency floor assumptions

🔥 For a world-class reusable package, compatibility matrix testing is not optional polish.  
It is part of the architecture contract.

## 7. `version_check` is narrow and should stay that way

`version_check 0.9.5` can be useful for very small build-time gates.

But it should not become a substitute for:

- explicit MSRV policy
- explicit feature docs
- explicit compatibility testing

Practical takeaway:

✅ okay as a tiny build helper  
❌ not a main compatibility strategy

## 8. Runtime capability negotiation should be typed and layered

Given everything already learned in this research, a healthy capability handshake likely needs to distinguish at least:

### Build capabilities

Examples:

- `remote_ssh`
- `sandbox_wasm`
- `c_api`
- `node_adapter`
- `sqlite_bundled`

### Runtime availability

Examples:

- configured/not configured
- platform-supported/not supported
- dependencies present/not present

### Session/client authority

Examples:

- viewer/controller
- clipboard read/write
- search
- attach
- open-link
- spawn route

🔥 If these collapse into one boolean blob, hosts will make bad assumptions.

## 9. Default features should stay minimal

The safest long-term package policy here looks like:

- minimal core defaults
- opt-in heavy leaves
- no surprise networking/sandbox/SSH/plugin runtimes in default builds

Why:

- smaller support surface
- easier semver reasoning
- easier security story
- easier embedding into other products

This is especially important because the package may later expose:

- Rust crates
- C ABI
- Node bindings
- standalone apps

Default features should not silently drag the whole world into each of those.

## 10. What I would actually build

For this terminal package I would shape compatibility like this:

### Crate topology

- keep heavy adapters in separate crates when possible
- keep core runtime crates lean

### Cargo features

- use only for real compile-time exclusions
- group them coarsely, not for every tiny option

### Runtime handshake

- `protocol_version`
- `build_capabilities`
- `runtime_capabilities`
- `session_role_capabilities`
- maybe `resource_limits`

### CI matrix

- default
- no-default-features where meaningful
- selected leaf-feature combos with `cargo-hack`
- MSRV with `cargo-msrv`
- dependency floors with `cargo-minimal-versions`

### Docs

- `document-features`
- clear compatibility matrix in release docs

## Practical recommendations

- ✅ Separate Cargo features from runtime capabilities
- ✅ Separate runtime capabilities from authority
- ✅ Prefer separate leaf crates over giant feature-soup core crates
- ✅ Use `cfg_aliases` for cfg hygiene
- ✅ Use `document-features` for public feature docs
- ✅ Use `target-lexicon` for honest target-aware behavior
- ✅ Treat `cargo-msrv`, `cargo-hack` and `cargo-minimal-versions` as compatibility-contract tools
- ⚠️ Keep default features minimal
- ⚠️ Keep `version_check` narrow
- ❌ Do not let compile-time flags become the only compatibility story
- ❌ Do not let runtime infer capability only from implicit build shape

## Sources

- [semver](https://docs.rs/semver)
- [cfg_aliases](https://docs.rs/cfg_aliases)
- [document-features](https://docs.rs/document-features)
- [target-lexicon](https://docs.rs/target-lexicon)
- [cargo-msrv](https://github.com/foresterre/cargo-msrv)
- [cargo-hack](https://github.com/taiki-e/cargo-hack)
- [cargo-minimal-versions](https://github.com/taiki-e/cargo-minimal-versions)
- [version_check](https://docs.rs/version_check)
